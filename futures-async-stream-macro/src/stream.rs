// SPDX-License-Identifier: Apache-2.0 OR MIT

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Block, ExprAsync, FnArg, Pat, PatIdent, PatType, Result, Signature, Stmt, Token, Type,
    parse::{Parse, ParseStream},
    parse_quote,
    punctuated::Punctuated,
    token,
    visit_mut::VisitMut as _,
};

use crate::{
    elision,
    parse::{self, Context, FnOrAsync, FnSig},
    utils::parse_as_empty,
    visitor::Visitor,
};

mod kw {
    syn::custom_keyword!(item);
    syn::custom_keyword!(ok);
    syn::custom_keyword!(error);
    syn::custom_keyword!(boxed);
    syn::custom_keyword!(boxed_local);
}

pub(crate) fn attribute(args: TokenStream, input: TokenStream, cx: Context) -> Result<TokenStream> {
    match parse::parse(input, cx)? {
        FnOrAsync::Fn(sig) => parse_fn(args, sig, cx),
        FnOrAsync::Async(mut expr, semi) => {
            parse_as_empty(&args)?;
            let mut tokens = parse_async(&mut expr, cx);
            if let Some(semi) = semi {
                semi.to_tokens(&mut tokens);
            }
            Ok(tokens)
        }
        FnOrAsync::NotAsync => unreachable!(),
    }
}

pub(crate) fn parse_async(expr: &mut ExprAsync, cx: Context) -> TokenStream {
    Visitor::new(cx.into()).visit_expr_async_mut(expr);
    make_gen_body(expr.capture.as_ref(), &expr.block, cx, None, false)
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
                    Self::Default => *self = Self::Boxed { send: true },
                    Self::Boxed { send: true } => bail!(i, "duplicate `boxed` argument"),
                    Self::Boxed { send: false } => {
                        bail!(i, "`boxed` and `boxed_local` may not be used at the same time");
                    }
                }
            } else if input.peek(kw::boxed_local) {
                let i: kw::boxed_local = input.parse()?;
                match self {
                    Self::Default => *self = Self::Boxed { send: false },
                    Self::Boxed { send: false } => bail!(i, "duplicate `boxed_local` argument"),
                    Self::Boxed { send: true } => {
                        bail!(i, "`boxed` and `boxed_local` may not be used at the same time");
                    }
                }
            } else {
                f(input)?;
            }

            if input.is_empty() {
                break;
            }
            let _: Token![,] = input.parse()?;
        }

        Ok(())
    }

    fn is_boxed(self) -> bool {
        matches!(self, Self::Boxed { .. })
    }
}

// Parses `= <value>` in `<name> = <value>` and returns value and span of name-value pair.
fn parse_value(
    input: ParseStream<'_>,
    name: &impl ToTokens,
    has_prev: bool,
) -> Result<(Type, TokenStream)> {
    if input.is_empty() {
        bail!(name, "expected `{0} = <type>`, found `{0}`", name.to_token_stream());
    }
    let eq_token: Token![=] = input.parse()?;
    if input.is_empty() {
        let span = quote!(#name #eq_token);
        bail!(span, "expected `{0} = <type>`, found `{0} =`", name.to_token_stream());
    }
    let value: Type = input.parse()?;
    let span = quote!(#name #value);
    if has_prev {
        bail!(span, "duplicate `{}` argument", name.to_token_stream())
    }
    Ok((value, span))
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
                item_ty = Some(parse_value(input, &i, item_ty.is_some())?.0);
                Ok(())
            } else if item_ty.is_none() {
                input.parse::<kw::item>().map(|_| unreachable!())
            } else {
                let token = input.parse::<TokenStream>()?;
                bail!(token, "unexpected argument: {}", token)
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
                ok = Some(parse_value(input, &i, ok.is_some())?.0);
                Ok(())
            } else if input.peek(kw::error) {
                // error = <Type>
                let i: kw::error = input.parse()?;
                error = Some(parse_value(input, &i, error.is_some())?.0);
                Ok(())
            } else if ok.is_none() {
                input.parse::<kw::ok>().map(|_| unreachable!())
            } else if error.is_none() {
                input.parse::<kw::error>().map(|_| unreachable!())
            } else {
                let token = input.parse::<TokenStream>()?;
                bail!(token, "unexpected argument: {}", token)
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
    Ok(match cx {
        Context::Stream => {
            let StreamArg { item_ty, boxed } = syn::parse2(args)?;
            let trait_ = quote! {
                ::futures_async_stream::__private::stream::Stream<Item = #item_ty>
            };
            parse_fn_inner(sig, cx, None, boxed.is_boxed(), |lifetimes| match boxed {
                ReturnTypeKind::Default => {
                    // Raw `impl` breaks syntax highlighting in some editors.
                    let impl_token = token::Impl::default();
                    quote! {
                        #impl_token #trait_ + #lifetimes
                    }
                }
                ReturnTypeKind::Boxed { send } => {
                    let send = if send {
                        Some(quote!(+ ::futures_async_stream::__private::Send))
                    } else {
                        None
                    };
                    quote! {
                        ::futures_async_stream::__private::Pin<Box<
                            dyn #trait_ #send + #lifetimes
                        >>
                    }
                }
            })
        }
        Context::TryStream => {
            let TryStreamArg { ok, error, boxed } = syn::parse2(args)?;
            parse_fn_inner(sig, cx, Some(&error), boxed.is_boxed(), |lifetimes| {
                let trait_ = quote! {
                    ::futures_async_stream::__private::stream::Stream<
                        Item = ::futures_async_stream::__private::Result<#ok, #error>
                    >
                };
                match boxed {
                    ReturnTypeKind::Default => {
                        // Raw `impl` breaks syntax highlighting in some editors.
                        let impl_token = token::Impl::default();
                        quote! {
                            #impl_token #trait_ + #lifetimes
                        }
                    }
                    ReturnTypeKind::Boxed { send } => {
                        let send = if send {
                            Some(quote!(+ ::futures_async_stream::__private::Send))
                        } else {
                            None
                        };
                        quote! {
                            ::futures_async_stream::__private::Pin<Box<
                                dyn #trait_ #send + #lifetimes
                            >>
                        }
                    }
                }
            })
        }
    })
}

fn parse_fn_inner(
    sig: FnSig,
    cx: Context,
    error: Option<&Type>,
    boxed: bool,
    return_ty: impl FnOnce(TokenStream) -> TokenStream,
) -> TokenStream {
    let FnSig { attrs, vis, sig, mut block, semi } = sig;
    let Signature { unsafety, abi, fn_token, ident, mut generics, inputs, .. } = sig;

    // Visit `#[for_await]`, `.await`, and `yield`.
    Visitor::new(cx.into()).visit_block_mut(&mut block);

    let (mut arguments, mut statements) = expand_async_body(inputs);
    statements.append(&mut block.stmts);
    block.stmts = statements;

    let body_inner = make_gen_body(Some(&<Token![move]>::default()), &block, cx, error, boxed);
    let mut body = TokenStream::new();
    block.brace_token.surround(&mut body, |tokens| {
        body_inner.to_tokens(tokens);
    });

    elision::unelide_lifetimes(&mut generics, &mut arguments);
    let where_clause = &generics.where_clause;
    let lifetimes = generics.lifetimes().map(|def| &def.lifetime);
    let return_ty = return_ty(quote!(#(#lifetimes +)*));

    let body = semi.map_or(body, ToTokens::into_token_stream);
    quote! {
        #(#attrs)*
        #vis #unsafety #abi #fn_token #ident #generics (#(#arguments),*) -> #return_ty
        #where_clause
        #body
    }
}

fn expand_async_body(inputs: Punctuated<FnArg, Token![,]>) -> (Vec<FnArg>, Vec<Stmt>) {
    let mut arguments = Vec::with_capacity(inputs.len());
    let mut statements = vec![];

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
    //          from_coroutine(#[coroutine] static move || {
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
                arguments.push(FnArg::Typed(PatType { attrs, pat, colon_token, ty }));
                continue;
            }

            let ident = def_site_ident!("__arg{}", i);

            // Construct the `let <pat> = __argN;` statement.
            statements.push(parse_quote!(let #pat = #ident;));

            let pat = Box::new(Pat::Ident(PatIdent {
                attrs: vec![],
                by_ref: None,
                mutability: Some(<Token![mut]>::default()),
                ident,
                subpat: None,
            }));
            arguments.push(FnArg::Typed(PatType { attrs, pat, colon_token, ty }));
        } else {
            arguments.push(argument);
        }
    }

    (arguments, statements)
}

fn make_gen_body(
    capture: Option<&Token![move]>,
    block: &Block,
    cx: Context,
    error: Option<&Type>,
    boxed: bool,
) -> TokenStream {
    let (gen_function, ret_value, ret_ty) = match cx {
        Context::Stream => (
            quote!(::futures_async_stream::__private::stream::from_coroutine),
            TokenStream::new(),
            quote!(()),
        ),
        Context::TryStream => {
            let error = error.map_or_else(|| quote!(_), ToTokens::to_token_stream);
            (
                quote!(::futures_async_stream::__private::try_stream::from_coroutine),
                quote!(::futures_async_stream::__private::Ok(())),
                quote!(::futures_async_stream::__private::Result<(), #error>),
            )
        }
    };

    let task_context = def_site_ident!("__task_context");
    let body = quote! {
        #gen_function(
            #[coroutine]
            static #capture |
                mut #task_context: ::futures_async_stream::__private::future::ResumeTy,
            | -> #ret_ty {
                let (): () = #block;

                // Ensure that this closure is a coroutine, even if it doesn't
                // have any `yield` statements.
                #[allow(unreachable_code)]
                {
                    return #ret_value;
                    loop {
                        #task_context = yield ::futures_async_stream::__private::Poll::Pending;
                    }
                }
            }
        )
    };

    if boxed { quote!(Box::pin(#body)) } else { body }
}
