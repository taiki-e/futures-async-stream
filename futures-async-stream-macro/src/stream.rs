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

#[allow(dead_code)] // fixed in latest nightly
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
                    Self::Default => *self = Self::Boxed { send: true },
                    Self::Boxed { send: true } => {
                        return Err(error!(i, "duplicate `boxed` argument"));
                    }
                    Self::Boxed { send: false } => {
                        return Err(error!(
                            i,
                            "`boxed` and `boxed_local` may not be used at the same time"
                        ));
                    }
                }
            } else if input.peek(kw::boxed_local) {
                let i: kw::boxed_local = input.parse()?;
                match self {
                    Self::Default => *self = Self::Boxed { send: false },
                    Self::Boxed { send: false } => {
                        return Err(error!(i, "duplicate `boxed_local` argument"));
                    }
                    Self::Boxed { send: true } => {
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

// Replace `prev` with `new`. Returns `Err` if `prev` is `Some`.
fn replace<T>(prev: &mut Option<T>, new: T, token: &impl ToTokens) -> Result<()> {
    if prev.replace(new).is_some() {
        Err(error!(token, "duplicate `{}` argument", token.to_token_stream()))
    } else {
        Ok(())
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
            if input.peek(kw::item) {
                // item = <Type>
                let i: kw::item = input.parse()?;
                let _: token::Eq = input.parse()?;
                replace(&mut item_ty, input.parse()?, &i)
            } else if item_ty.is_none() {
                input.parse::<kw::item>().map(|_| unreachable!())
            } else {
                let token = input.parse::<TokenStream>()?;
                Err(error!(token, "unexpected argument: {}", token))
            }
        })?;

        if let Some(item_ty) = item_ty {
            Ok(Self { item_ty, boxed })
        } else {
            input.parse::<kw::item>().map(|_| unreachable!())
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
                replace(&mut ok, input.parse()?, &i)
            } else if input.peek(kw::error) {
                // error = <Type>
                let i: kw::error = input.parse()?;
                let _: token::Eq = input.parse()?;
                replace(&mut error, input.parse()?, &i)
            } else if ok.is_none() {
                input.parse::<kw::ok>().map(|_| unreachable!())
            } else if error.is_none() {
                input.parse::<kw::error>().map(|_| unreachable!())
            } else {
                let token = input.parse::<TokenStream>()?;
                Err(error!(token, "unexpected argument: {}", token))
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
    mut sig: FnSig,
    cx: Context,
    error: Option<&Type>,
    boxed: bool,
    return_ty: impl FnOnce(TokenStream) -> TokenStream,
) -> Result<TokenStream> {
    validate_sig(Some(&sig), &sig.attrs, cx)?;

    let FnSig { attrs, vis, sig, block, semi } = &mut sig;
    let has_self = sig.receiver().is_some();
    transform_sig(cx, sig, has_self, true);
    let Signature { unsafety, abi, fn_token, ident, generics, inputs, .. } = sig;
    let where_clause = &generics.where_clause;

    // Visit `#[for_await]`, `.await`, and `yield`.
    Visitor::new(cx.into()).visit_block_mut(block);

    let (mut arguments, mut statements) = expand_async_body(&inputs);
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

fn expand_async_body(inputs: &Punctuated<FnArg, token::Comma>) -> (Vec<FnArg>, Vec<Stmt>) {
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
    for (i, argument) in inputs.iter().cloned().enumerate() {
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

// Input:
//     async fn f<T>(&self, x: &T) -> Ret;
//
// Output:
//     fn f<'life0, 'life1, 'async_stream, T>(
//         &'life0 self,
//         x: &'life1 T,
//     ) -> Pin<Box<dyn Future<Output = Ret> + Send + 'async_stream>>
//     where
//         'life0: 'async_stream,
//         'life1: 'async_stream,
//         T: 'async_stream,
//         Self: Sync + 'async_stream;
fn transform_sig(_: Context, sig: &mut Signature, has_self: bool, is_local: bool) {
    use crate::lifetime::CollectLifetimes;
    use quote::{format_ident, quote, quote_spanned, ToTokens};
    use syn::{
        parse_quote, punctuated::Punctuated, visit_mut::VisitMut, Block, FnArg, GenericParam,
        Generics, Ident, ImplItem, Lifetime, Pat, PatIdent, Path, Receiver, ReturnType, Signature,
        Stmt, Token, TraitItem, Type, TypeParam, TypeParamBound, WhereClause,
    };

    sig.fn_token.span = sig.asyncness.take().unwrap().span;

    let ret = match &sig.output {
        ReturnType::Default => quote!(()),
        ReturnType::Type(_, ret) => quote!(#ret),
    };

    let mut lifetimes = CollectLifetimes::new();
    for arg in sig.inputs.iter_mut() {
        match arg {
            FnArg::Receiver(arg) => lifetimes.visit_receiver_mut(arg),
            FnArg::Typed(arg) => lifetimes.visit_type_mut(&mut arg.ty),
        }
    }

    let where_clause = sig.generics.where_clause.get_or_insert_with(|| WhereClause {
        where_token: token::Where::default(),
        predicates: Punctuated::new(),
    });
    for param in sig.generics.params.iter()
    //.chain(context.lifetimes(&lifetimes.explicit))
    {
        match param {
            GenericParam::Type(param) => {
                let param = &param.ident;
                where_clause.predicates.push(parse_quote!(#param: 'async_stream));
            }
            GenericParam::Lifetime(param) => {
                let param = &param.lifetime;
                where_clause.predicates.push(parse_quote!(#param: 'async_stream));
            }
            GenericParam::Const(_) => {}
        }
    }
    for elided in lifetimes.elided {
        sig.generics.params.push(parse_quote!(#elided));
        where_clause.predicates.push(parse_quote!(#elided: 'async_stream));
    }
    sig.generics.params.push(parse_quote!('async_stream));
    if has_self {
        // let bound: Ident = match sig.inputs.iter().next() {
        //     Some(FnArg::Receiver(Receiver { reference: Some(_), mutability: None, .. })) => {
        //         parse_quote!(Sync)
        //     }
        //     Some(FnArg::Typed(arg))
        //         if match (arg.pat.as_ref(), arg.ty.as_ref()) {
        //             (Pat::Ident(pat), Type::Reference(ty)) => {
        //                 pat.ident == "self" && ty.mutability.is_none()
        //             }
        //             _ => false,
        //         } =>
        //     {
        //         parse_quote!(Sync)
        //     }
        //     _ => parse_quote!(Send),
        // };
        // let assume_bound = match context {
        //     Context::Trait { supertraits, .. } => !has_default || has_bound(supertraits, &bound),
        //     Context::Impl { .. } => true,
        // };
        // where_clause.predicates.push(if assume_bound || is_local {
        //     parse_quote!(Self: 'async_stream)
        // } else {
        //     parse_quote!(Self: ::core::marker::#bound + 'async_stream)
        // });
        where_clause.predicates.push(parse_quote!(Self: 'async_stream));
    }

    for (i, arg) in sig.inputs.iter_mut().enumerate() {
        match arg {
            FnArg::Receiver(Receiver { reference: Some(_), .. }) => {}
            FnArg::Receiver(arg) => arg.mutability = None,
            FnArg::Typed(arg) => {
                if let Pat::Ident(ident) = &mut *arg.pat {
                    ident.by_ref = None;
                    ident.mutability = None;
                } else {
                    let positional = positional_arg(i);
                    *arg.pat = parse_quote!(#positional);
                }
            }
        }
    }

    fn positional_arg(i: usize) -> Ident {
        format_ident!("__arg{}", i)
    }

    let bounds =
        if is_local { quote!('async_stream) } else { quote!(::core::marker::Send + 'async_stream) };

    sig.output = parse_quote! {
        -> ::core::pin::Pin<Box<
            dyn ::core::future::Future<Output = #ret> + #bounds
        >>
    };
}
