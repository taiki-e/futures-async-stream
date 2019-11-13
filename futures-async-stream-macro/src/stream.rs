use proc_macro2::{Group, TokenStream, TokenTree};
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::{self, Comma},
    visit_mut::VisitMut,
    *,
};

use crate::{
    elision,
    utils::{block, first_last, respan},
    visitor::{Stream, Visitor},
};

// =================================================================================================
// async_stream

pub(super) fn attribute(args: TokenStream, input: TokenStream) -> Result<TokenStream> {
    parse_async_stream_fn(args, input)
}

mod kw {
    syn::custom_keyword!(item);
    syn::custom_keyword!(boxed);
    syn::custom_keyword!(boxed_local);
}

// item = <Type>
struct Item(Type);

impl Parse for Item {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let _: kw::item = input.parse()?;
        let _: Token![=] = input.parse()?;
        input.parse().map(Self)
    }
}

#[derive(Clone, Copy)]
pub(super) enum ReturnTypeKind {
    // impl Stream<Item = ..> $(+ $lifetime)?
    Default,
    // Pin<Box<dyn Stream<Item = ..> (+ Send)? $(+ $lifetime)?>>
    Boxed { send: bool },
}

impl ReturnTypeKind {
    pub(super) fn parse_or_else<F>(&mut self, input: ParseStream<'_>, mut f: F) -> Result<()>
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
                            "`boxed` and `boxed_local` cannot be used at the same time."
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
                            "`boxed` and `boxed_local` cannot be used at the same time."
                        ));
                    }
                }
            } else {
                f(input)?;
            }

            if !input.is_empty() {
                let _: Comma = input.parse()?;
            }
        }

        Ok(())
    }
}

struct Args {
    item: Type,
    boxed: ReturnTypeKind,
}

impl Parse for Args {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut item = None;
        let mut boxed = ReturnTypeKind::Default;
        boxed.parse_or_else(input, |input| {
            let i: Item = input.parse()?;
            if item.is_some() {
                Err(error!(i.0, "duplicate `item` argument"))
            } else {
                item = Some(i.0);
                Ok(())
            }
        })?;

        if let Some(item) = item {
            Ok(Self { item, boxed })
        } else {
            let _: kw::item = input.parse()?;
            unreachable!()
        }
    }
}

pub(super) struct FnSig {
    pub(super) attrs: Vec<Attribute>,
    pub(super) vis: Visibility,
    pub(super) sig: Signature,
    pub(super) block: Block,
    pub(super) semi: Option<Token![;]>,
}

impl FnSig {
    pub(super) fn parse(input: TokenStream, boxed: ReturnTypeKind) -> Result<Self> {
        match boxed {
            ReturnTypeKind::Default => syn::parse2(input).map(ItemFn::into),
            ReturnTypeKind::Boxed { .. } => {
                let input2 = input.clone();
                syn::parse2(input)
                    .map(ItemFn::into)
                    .or_else(|e| syn::parse2(input2).map(TraitItemMethod::into).map_err(|_e| e))
            }
        }
    }
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

pub(super) fn validate_async_stream_fn(item: &FnSig) -> Result<()> {
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
            Type::Tuple(TypeTuple { elems, .. }) if elems.is_empty() => {}
            _ => return Err(error!(ty, "async stream must return the unit type")),
        }
    }

    Ok(())
}

pub(super) fn expand_async_body(inputs: Punctuated<FnArg, Comma>) -> (Vec<FnArg>, Vec<Local>) {
    let mut arguments: Vec<FnArg> = Vec::new();
    let mut statements: Vec<Local> = Vec::new();

    // Desugar `async fn`
    // from:
    //
    //      #[async_stream(item = u32)]
    //      async fn foo(ref a: u32) {
    //          // ...
    //      }
    //
    // into:
    //
    //      fn foo(__arg0: u32) -> impl Stream<Item = u32> {
    //          from_generator(static move || {
    //              let ref a = __arg0;
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

            let ident = format_ident!("__arg{}", i);

            let local = Local {
                attrs: Vec::new(),
                let_token: token::Let::default(),
                pat: *pat,
                init: Some((
                    token::Eq::default(),
                    Box::new(Expr::Path(ExprPath {
                        attrs: Vec::new(),
                        qself: None,
                        path: ident.clone().into(),
                    })),
                )),
                semi_token: token::Semi::default(),
            };
            statements.push(local);

            let pat = Box::new(Pat::Ident(PatIdent {
                attrs: Vec::new(),
                by_ref: None,
                mutability: None,
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

pub(super) fn make_gen_body(
    statements: &[Local],
    block: &Block,
    gen_function: &TokenStream,
    ret_value: &TokenStream,
    ret_ty: &TokenStream,
) -> TokenStream {
    let block_inner = quote! {
        #(#statements)*
        #block
    };
    let mut result = TokenStream::new();
    block.brace_token.surround(&mut result, |tokens| {
        block_inner.to_tokens(tokens);
    });
    token::Semi([block.brace_token.span]).to_tokens(&mut result);

    let gen_body_inner = quote! {
        let (): () = #result

        // Ensure that this closure is a generator, even if it doesn't
        // have any `yield` statements.
        #[allow(unreachable_code)]
        {
            return #ret_value;
            loop { yield ::futures_async_stream::reexport::task::Poll::Pending }
        }
    };
    let mut gen_body = TokenStream::new();
    block.brace_token.surround(&mut gen_body, |tokens| {
        gen_body_inner.to_tokens(tokens);
    });

    quote! {
        #gen_function(static move || -> #ret_ty #gen_body)
    }
}

fn parse_async_stream_fn(args: TokenStream, input: TokenStream) -> Result<TokenStream> {
    let args: Args = syn::parse2(args)?;
    let item = FnSig::parse(input, args.boxed)?;

    validate_async_stream_fn(&item)?;
    Ok(expand_async_stream_fn(item, &args))
}

fn expand_async_stream_fn(item: FnSig, args: &Args) -> TokenStream {
    let FnSig { attrs, vis, sig, mut block, semi } = item;
    let Signature { unsafety, abi, fn_token, ident, mut generics, inputs, .. } = sig;
    let where_clause = &generics.where_clause;

    let (mut arguments, statements) = expand_async_body(inputs);

    // Visit `#[for_await]` and `.await`.
    Visitor::new(Stream).visit_block_mut(&mut block);

    let item = &args.item;

    // Give the invocation of the `from_generator` function the same span as the `item`
    // as currently errors related to it being a result are targeted here. Not
    // sure if more errors will highlight this function call...
    let output_span = first_last(item);
    let gen_function = quote!(::futures_async_stream::stream::from_generator);
    let gen_function = respan(gen_function, output_span);
    let mut body_inner = make_gen_body(&statements, &block, &gen_function, &quote!(), &quote!(()));

    if let ReturnTypeKind::Boxed { .. } = args.boxed {
        let body = quote! { Box::pin(#body_inner) };
        body_inner = respan(body, output_span);
    }

    let mut body = TokenStream::new();
    block.brace_token.surround(&mut body, |tokens| {
        body_inner.to_tokens(tokens);
    });

    elision::unelide_lifetimes(&mut generics.params, &mut arguments);
    let lifetimes = generics.lifetimes().map(|l| &l.lifetime);

    let return_ty = match args.boxed {
        ReturnTypeKind::Default => {
            // Raw `impl` breaks syntax highlighting in some editors.
            let impl_token = token::Impl::default();
            quote! {
                #impl_token ::futures_async_stream::reexport::Stream<Item = #item> + #(#lifetimes +)*
            }
        }
        ReturnTypeKind::Boxed { send } => {
            let send = if send {
                quote!(+ ::futures_async_stream::reexport::marker::Send)
            } else {
                TokenStream::new()
            };
            quote! {
                ::futures_async_stream::reexport::pin::Pin<
                    Box<dyn ::futures_async_stream::reexport::Stream<Item = #item> #send + #(#lifetimes +)*>
                >
            }
        }
    };
    let return_ty = respan(return_ty, output_span);

    // FIXME
    let body = semi.map_or_else(|| body, ToTokens::into_token_stream);
    quote! {
        #(#attrs)*
        #vis #unsafety #abi
        #fn_token #ident #generics (#(#arguments),*)
            -> #return_ty
            #where_clause
        #body
    }
}

// =================================================================================================
// async_stream_block

pub(super) fn block_macro(input: TokenStream) -> Result<TokenStream> {
    syn::parse2(replace_for_await(input)).map(expand_async_stream_block)
}

fn expand_async_stream_block(mut expr: Expr) -> TokenStream {
    Visitor::new(Stream).visit_expr_mut(&mut expr);

    let gen_function = quote!(::futures_async_stream::stream::from_generator);
    make_gen_body(&[], &block(vec![Stmt::Expr(expr)]), &gen_function, &quote!(), &quote!(()))
}

fn replace_for_await(input: TokenStream) -> TokenStream {
    let mut input = input.into_iter().peekable();
    let mut tokens = Vec::new();

    while let Some(token) = input.next() {
        match token {
            TokenTree::Ident(ident) => {
                match input.peek() {
                    Some(TokenTree::Ident(next)) if ident == "for" && next == "await" => {
                        let next = format_ident!("for_{}", next);
                        tokens.extend(quote!(#[#next]));
                        let _ = input.next();
                    }
                    _ => {}
                }
                tokens.push(ident.into());
            }
            TokenTree::Group(group) => {
                let stream = replace_for_await(group.stream());
                tokens.push(Group::new(group.delimiter(), stream).into());
            }
            _ => tokens.push(token),
        }
    }

    tokens.into_iter().collect()
}
