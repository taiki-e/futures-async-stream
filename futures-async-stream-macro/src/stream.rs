use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token,
    visit_mut::VisitMut,
    Attribute, Block, Expr, ExprAsync, FnArg, Item, ItemFn, Pat, PatIdent, PatType, Result,
    ReturnType, Signature, Stmt, TraitItemMethod, Type, Visibility,
};

use crate::{
    elision,
    utils::{parse_as_empty, SliceExt, TASK_CONTEXT},
    visitor::{Scope, Visitor},
};

mod kw {
    syn::custom_keyword!(item);
    syn::custom_keyword!(ok);
    syn::custom_keyword!(error);
    syn::custom_keyword!(boxed);
    syn::custom_keyword!(boxed_local);
}

pub(crate) fn attribute(args: TokenStream, input: TokenStream, cx: Context) -> Result<TokenStream> {
    match syn::parse2(input.clone()) {
        Ok(Stmt::Item(Item::Fn(item))) => parse_fn(args, item.into(), cx),
        Ok(Stmt::Expr(Expr::Async(mut expr))) | Ok(Stmt::Semi(Expr::Async(mut expr), _)) => {
            parse_as_empty(&args)?;
            parse_async(&mut expr, cx)
        }
        _ => {
            if let Ok(item) = syn::parse2::<TraitItemMethod>(input.clone()) {
                parse_fn(args, item.into(), cx)
            } else if let Ok(mut expr) = syn::parse2::<ExprAsync>(input.clone()) {
                parse_as_empty(&args)?;
                parse_async(&mut expr, cx)
            } else {
                Err(error!(
                    input,
                    "#[{}] attribute may not be used on async functions or async blocks",
                    cx.as_str()
                ))
            }
        }
    }
}

pub(crate) fn parse_async(expr: &mut ExprAsync, cx: Context) -> Result<TokenStream> {
    validate_sig(None, &expr.attrs, cx)?;

    Visitor::new(cx.into()).visit_expr_async_mut(expr);
    Ok(make_gen_body(expr.capture, &expr.block, cx, None, false))
}

#[derive(Copy, Clone)]
pub(crate) enum Context {
    Stream,
    TryStream,
}

impl Context {
    fn as_str(self) -> &'static str {
        match self {
            Self::Stream => "stream",
            Self::TryStream => "try_stream",
        }
    }
}

impl From<Context> for Scope {
    fn from(other: Context) -> Self {
        match other {
            Context::Stream => Self::Stream,
            Context::TryStream => Self::TryStream,
        }
    }
}

#[derive(Clone, Copy)]
enum ReturnTypeKind {
    // impl Stream<Item = ..> $(+ $lifetime)?
    Default,
    // Pin<Box<dyn Stream<Item = ..> (+ Send)? $(+ $lifetime)?>>
    Boxed { send: bool },
}

impl ReturnTypeKind {
    fn parse_or_else<F>(&mut self, input: ParseStream<'_>, mut f: F) -> Result<()>
    where
        F: FnMut(ParseStream<'_>) -> Result<()>,
    {
        while !input.is_empty() {
            if input.peek(kw::boxed) {
                let i: kw::boxed = input.parse()?;
                match self {
                    ReturnTypeKind::Default => *self = ReturnTypeKind::Boxed { send: true },
                    ReturnTypeKind::Boxed { send: true } => {
                        return Err(error!(i, "duplicate `boxed` argument"));
                    }
                    ReturnTypeKind::Boxed { send: false } => {
                        return Err(error!(
                            i,
                            "`boxed` and `boxed_local` may not be used at the same time"
                        ));
                    }
                }
            } else if input.peek(kw::boxed_local) {
                let i: kw::boxed_local = input.parse()?;
                match self {
                    ReturnTypeKind::Default => *self = ReturnTypeKind::Boxed { send: false },
                    ReturnTypeKind::Boxed { send: false } => {
                        return Err(error!(i, "duplicate `boxed_local` argument"));
                    }
                    ReturnTypeKind::Boxed { send: true } => {
                        return Err(error!(
                            i,
                            "`boxed` and `boxed_local` may not be used at the same time"
                        ));
                    }
                }
            } else {
                f(input)?;
            }

            if !input.is_empty() {
                let _: token::Comma = input.parse()?;
            }
        }

        Ok(())
    }

    fn is_boxed(self) -> bool {
        if let Self::Boxed { .. } = self { true } else { false }
    }
}

struct FnSig {
    attrs: Vec<Attribute>,
    vis: Visibility,
    sig: Signature,
    block: Block,
    semi: Option<token::Semi>,
}

impl From<ItemFn> for FnSig {
    fn from(item: ItemFn) -> Self {
        Self { attrs: item.attrs, vis: item.vis, sig: item.sig, block: *item.block, semi: None }
    }
}

impl From<TraitItemMethod> for FnSig {
    fn from(item: TraitItemMethod) -> Self {
        if let Some(block) = item.default {
            Self { attrs: item.attrs, vis: Visibility::Inherited, sig: item.sig, block, semi: None }
        } else {
            assert!(item.semi_token.is_some());
            Self {
                attrs: item.attrs,
                vis: Visibility::Inherited,
                sig: item.sig,
                block: Block { brace_token: token::Brace::default(), stmts: Vec::new() },
                semi: item.semi_token,
            }
        }
    }
}

struct StreamArg {
    item_ty: Type,
    boxed: ReturnTypeKind,
}

impl Parse for StreamArg {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut item_ty = None;
        let mut boxed = ReturnTypeKind::Default;
        boxed.parse_or_else(input, |input| {
            // item = <Type>
            let i: kw::item = input.parse()?;
            let _: token::Eq = input.parse()?;
            let ty: Type = input.parse()?;

            if item_ty.replace(ty).is_some() {
                Err(error!(i, "duplicate `item` argument"))
            } else {
                Ok(())
            }
        })?;

        if let Some(item_ty) = item_ty {
            Ok(Self { item_ty, boxed })
        } else {
            let _: kw::item = input.parse()?;
            unreachable!()
        }
    }
}

struct TryStreamArg {
    ok: Type,
    error: Type,
    boxed: ReturnTypeKind,
}

impl Parse for TryStreamArg {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut ok = None;
        let mut error = None;
        let mut boxed = ReturnTypeKind::Default;
        boxed.parse_or_else(input, |input| {
            if input.peek(kw::ok) {
                // ok = <Type>
                let i: kw::ok = input.parse()?;
                let _: token::Eq = input.parse()?;
                let ty: Type = input.parse()?;

                if ok.replace(ty).is_some() {
                    Err(error!(i, "duplicate `ok` argument"))
                } else {
                    Ok(())
                }
            } else {
                // error = <Type>
                let i: kw::error = input.parse()?;
                let _: token::Eq = input.parse()?;
                let ty: Type = input.parse()?;

                if error.replace(ty).is_some() {
                    Err(error!(i, "duplicate `error` argument"))
                } else {
                    Ok(())
                }
            }
        })?;

        match (ok, error) {
            (Some(ok), Some(error)) => Ok(Self { ok, error, boxed }),
            (Some(_), None) => input.parse::<kw::error>().map(|_| unreachable!()),
            (None, _) => input.parse::<kw::ok>().map(|_| unreachable!()),
        }
    }
}

fn parse_fn(args: TokenStream, sig: FnSig, cx: Context) -> Result<TokenStream> {
    match cx {
        Context::Stream => {
            let StreamArg { item_ty, boxed } = syn::parse2(args)?;

            parse_fn_inner(sig, cx, None, boxed.is_boxed(), |lifetimes| match boxed {
                ReturnTypeKind::Default => {
                    // Raw `impl` breaks syntax highlighting in some editors.
                    let impl_token = token::Impl::default();
                    quote! {
                        #impl_token ::futures_async_stream::__reexport::stream::Stream<Item = #item_ty> + #lifetimes
                    }
                }
                ReturnTypeKind::Boxed { send } => {
                    let send = if send {
                        quote!(+ ::futures_async_stream::__reexport::marker::Send)
                    } else {
                        TokenStream::new()
                    };
                    quote! {
                        ::futures_async_stream::__reexport::pin::Pin<Box<
                            dyn ::futures_async_stream::__reexport::stream::Stream<Item = #item_ty> #send + #lifetimes
                        >>
                    }
                }
            })
        }
        Context::TryStream => {
            let TryStreamArg { ok, error, boxed } = syn::parse2(args)?;

            parse_fn_inner(sig, cx, Some(&error), boxed.is_boxed(), |lifetimes| {
                match boxed {
                    ReturnTypeKind::Default => {
                        // Raw `impl` breaks syntax highlighting in some editors.
                        let impl_token = token::Impl::default();
                        quote! {
                            #impl_token ::futures_async_stream::__reexport::stream::Stream<
                                Item = ::futures_async_stream::__reexport::result::Result<#ok, #error>
                            > + #lifetimes
                        }
                    }
                    ReturnTypeKind::Boxed { send } => {
                        let send = if send {
                            quote!(+ ::futures_async_stream::__reexport::marker::Send)
                        } else {
                            TokenStream::new()
                        };
                        quote! {
                            ::futures_async_stream::__reexport::pin::Pin<Box<
                                dyn ::futures_async_stream::__reexport::stream::Stream<
                                    Item = ::futures_async_stream::__reexport::result::Result<#ok, #error>
                                > #send + #lifetimes
                            >>
                        }
                    }
                }
            })
        }
    }
}

fn parse_fn_inner(
    sig: FnSig,
    cx: Context,
    error: Option<&Type>,
    boxed: bool,
    return_ty: impl FnOnce(TokenStream) -> TokenStream,
) -> Result<TokenStream> {
    validate_sig(Some(&sig), &sig.attrs, cx)?;

    let FnSig { attrs, vis, sig, mut block, semi } = sig;
    let Signature { unsafety, abi, fn_token, ident, mut generics, inputs, .. } = sig;
    let where_clause = &generics.where_clause;

    // Visit `#[for_await]`, `.await`, and `yield`.
    Visitor::new(cx.into()).visit_block_mut(&mut block);

    let (mut arguments, mut statements) = expand_async_body(inputs);
    statements.append(&mut block.stmts);
    block.stmts = statements;

    let body_inner = make_gen_body(Some(token::Move::default()), &block, cx, error, boxed);
    let mut body = TokenStream::new();
    block.brace_token.surround(&mut body, |tokens| {
        body_inner.to_tokens(tokens);
    });

    elision::unelide_lifetimes(&mut generics.params, &mut arguments);
    let lifetimes = generics.lifetimes().map(|l| &l.lifetime);
    let return_ty = return_ty(quote!(#(#lifetimes +)*));

    let body = semi.map_or(body, ToTokens::into_token_stream);
    Ok(quote! {
        #(#attrs)*
        #vis #unsafety #abi #fn_token #ident #generics (#(#arguments),*) -> #return_ty
        #where_clause
        #body
    })
}

fn validate_sig(item: Option<&FnSig>, attrs: &[Attribute], cx: Context) -> Result<()> {
    if let Some(item) = item {
        if item.sig.asyncness.is_none() {
            return Err(error!(item.sig.fn_token, "async stream must be declared as async"));
        }
        if let Some(constness) = item.sig.constness {
            // This line is currently unreachable.
            // `async const fn` and `const async fn` is rejected by syn.
            // `const fn` is rejected by the previous statements.
            return Err(error!(constness, "async stream may not be const"));
        }
        if let Some(variadic) = &item.sig.variadic {
            return Err(error!(variadic, "async stream may not be variadic"));
        }

        if let ReturnType::Type(_, ty) = &item.sig.output {
            match &**ty {
                Type::Tuple(ty) if ty.elems.is_empty() => {}
                _ => return Err(error!(ty, "async stream must return the unit type")),
            }
        }
    }

    let (duplicate, another) = match cx {
        Context::Stream => ("stream", "try_stream"),
        Context::TryStream => ("try_stream", "stream"),
    };
    if let Some(attr) = attrs.find(duplicate) {
        Err(error!(attr, "duplicate #[{}] attribute", duplicate))
    } else if let Some(attr) = attrs.find(another) {
        Err(error!(attr, "#[stream] and #[try_stream] may not be used at the same time"))
    } else {
        Ok(())
    }
}

fn expand_async_body(inputs: Punctuated<FnArg, token::Comma>) -> (Vec<FnArg>, Vec<Stmt>) {
    let mut arguments: Vec<FnArg> = Vec::new();
    let mut statements: Vec<Stmt> = Vec::new();

    // Desugar `async fn`
    // from:
    //
    //      #[stream(item = u32)]
    //      async fn foo(self: <ty>, ref <ident>: <ty>) {
    //          // ...
    //      }
    //
    // into:
    //
    //      fn foo(self: <ty>, mut __arg1: <ty>) -> impl Stream<Item = u32> {
    //          from_generator(static move || {
    //              let ref <ident> = __arg1;
    //
    //              // ...
    //          })
    //      }
    //
    // We notably skip everything related to `self` which typically doesn't have
    // many patterns with it and just gets captured naturally.
    for (i, argument) in inputs.into_iter().enumerate() {
        if let FnArg::Typed(PatType { attrs, pat, ty, colon_token }) = argument {
            let captured_naturally = match &*pat {
                // `self: Box<Self>` will get captured naturally
                Pat::Ident(PatIdent { ident, .. }) if ident == "self" => true,
                // `ref a: B` (or some similar pattern)
                Pat::Ident(PatIdent { by_ref: Some(_), .. }) => false,
                // Other arguments get captured naturally
                _ => true,
            };
            if captured_naturally {
                arguments.push(FnArg::Typed(PatType { attrs, pat, ty, colon_token }));
                continue;
            }

            let ident = def_site_ident!("__arg{}", i);

            // Construct the `let <pat> = __argN;` statement.
            statements.push(syn::parse_quote!(let #pat = #ident;));

            let pat = Box::new(Pat::Ident(PatIdent {
                attrs: Vec::new(),
                by_ref: None,
                mutability: Some(token::Mut::default()),
                ident,
                subpat: None,
            }));
            arguments.push(FnArg::Typed(PatType { attrs, pat, ty, colon_token }));
        } else {
            arguments.push(argument);
        }
    }

    (arguments, statements)
}

fn make_gen_body(
    capture: Option<token::Move>,
    block: &Block,
    cx: Context,
    error: Option<&Type>,
    boxed: bool,
) -> TokenStream {
    let (gen_function, ret_value, ret_ty) = match cx {
        Context::Stream => (
            quote!(::futures_async_stream::__reexport::stream::from_generator),
            TokenStream::new(),
            quote!(()),
        ),
        Context::TryStream => {
            let error = error.map_or_else(|| quote!(_), ToTokens::to_token_stream);
            (
                quote!(::futures_async_stream::__reexport::try_stream::from_generator),
                quote!(::futures_async_stream::__reexport::result::Result::Ok(())),
                quote!(::futures_async_stream::__reexport::result::Result<(), #error>),
            )
        }
    };

    let task_context = def_site_ident!(TASK_CONTEXT);
    let body = quote! {
        #gen_function(
            static #capture |mut #task_context: ::futures_async_stream::__reexport::future::ResumeTy| -> #ret_ty {
                let (): () = #block;

                // Ensure that this closure is a generator, even if it doesn't
                // have any `yield` statements.
                #[allow(unreachable_code)]
                {
                    return #ret_value;
                    loop { #task_context = yield ::futures_async_stream::__reexport::task::Poll::Pending }
                }
            }
        )
    };

    if boxed { quote!(Box::pin(#body)) } else { body }
}
