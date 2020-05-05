use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    token,
    visit_mut::VisitMut,
    Expr, ExprAsync, Item, Result, Signature, Stmt, Token, TraitItemMethod, Type,
};

use crate::{
    elision,
    stream::{expand_async_body, make_gen_body, validate_stream_fn, FnSig, ReturnTypeKind},
    utils::{block, first_last, parse_as_empty, respan},
    visitor::{TryStream, Visitor},
};

// =================================================================================================
// try_stream

pub(crate) fn attribute(args: TokenStream, input: TokenStream) -> Result<TokenStream> {
    let stmt = syn::parse2(input.clone());
    match stmt {
        Ok(Stmt::Item(Item::Fn(item))) => parse_try_stream_fn(args, item.into()),
        Ok(Stmt::Expr(Expr::Async(mut expr))) | Ok(Stmt::Semi(Expr::Async(mut expr), _)) => {
            parse_as_empty(&args)?;
            Ok(expand_try_stream_block(&mut expr))
        }
        _ => {
            if let Ok(item) = syn::parse2::<TraitItemMethod>(input.clone()) {
                parse_try_stream_fn(args, item.into())
            } else {
                Err(error!(
                    input,
                    "#[try_stream] attribute may not be used on async functions or async blocks"
                ))
            }
        }
    }
}

mod kw {
    syn::custom_keyword!(ok);
    syn::custom_keyword!(error);
}

struct Args {
    ok: Type,
    error: Type,
    boxed: ReturnTypeKind,
}

impl Parse for Args {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        // ok = <Type>
        struct Ok_(Type);
        impl Parse for Ok_ {
            fn parse(input: ParseStream<'_>) -> Result<Self> {
                let _: kw::ok = input.parse()?;
                let _: Token![=] = input.parse()?;
                input.parse().map(Self)
            }
        }

        // error = <Type>
        struct Error_(Type);
        impl Parse for Error_ {
            fn parse(input: ParseStream<'_>) -> Result<Self> {
                let _: kw::error = input.parse()?;
                let _: Token![=] = input.parse()?;
                input.parse().map(Self)
            }
        }

        let mut ok = None;
        let mut error = None;
        let mut boxed = ReturnTypeKind::Default;
        boxed.parse_or_else(input, |input| {
            if input.peek(kw::ok) {
                let i: Ok_ = input.parse()?;
                if ok.is_some() {
                    Err(error!(i.0, "duplicate `ok` argument"))
                } else {
                    ok = Some(i.0);
                    Ok(())
                }
            } else {
                let i: Error_ = input.parse()?;
                if error.is_some() {
                    Err(error!(i.0, "duplicate `error` argument"))
                } else {
                    error = Some(i.0);
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

fn parse_try_stream_fn(args: TokenStream, item: FnSig) -> Result<TokenStream> {
    let args: Args = syn::parse2(args)?;

    validate_stream_fn(&item)?;
    Ok(expand_try_stream_fn(item, &args))
}

fn expand_try_stream_fn(item: FnSig, args: &Args) -> TokenStream {
    let FnSig { attrs, vis, sig, mut block, semi } = item;
    let Signature { unsafety, abi, fn_token, ident, mut generics, inputs, .. } = sig;
    let where_clause = &generics.where_clause;

    let (mut arguments, mut statements) = expand_async_body(inputs);

    // Visit `#[for_await]` and `.await`.
    Visitor::new(TryStream).visit_block_mut(&mut block);

    let ok = &args.ok;
    let error = &args.error;

    // Give the invocation of the `from_generator` function the same span as the `item_ty`
    // as currently errors related to it being a result are targeted here. Not
    // sure if more errors will highlight this function call...
    let output_span = first_last(ok);
    let gen_function = quote!(::futures_async_stream::__reexport::try_stream::from_generator);
    let gen_function = respan(gen_function, output_span);
    let ret_ty = quote!(::futures_async_stream::__reexport::result::Result<(), #error>);
    statements.append(&mut block.stmts);
    block.stmts = statements;
    let mut body_inner = make_gen_body(
        Some(token::Move::default()),
        &block,
        &gen_function,
        &quote!(Ok(())),
        &ret_ty,
    );

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
                #impl_token ::futures_async_stream::__reexport::stream::Stream<
                    Item = ::futures_async_stream::__reexport::result::Result<#ok, #error>
                > + #(#lifetimes +)*
            }
        }
        ReturnTypeKind::Boxed { send } => {
            let send = if send {
                quote!(+ ::futures_async_stream::__reexport::marker::Send)
            } else {
                TokenStream::new()
            };
            quote! {
                ::futures_async_stream::__reexport::pin::Pin<
                    Box<dyn ::futures_async_stream::__reexport::stream::Stream<
                        Item = ::futures_async_stream::__reexport::result::Result<#ok, #error>
                    > #send + #(#lifetimes +)*>
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
// try_stream_block

pub(crate) fn block_macro(input: TokenStream) -> Result<TokenStream> {
    let mut expr = syn::parse2(input)?;

    Visitor::new(TryStream).visit_expr_mut(&mut expr);

    let gen_function = quote!(::futures_async_stream::__reexport::try_stream::from_generator);
    let ret_ty = quote!(::futures_async_stream::__reexport::result::Result<(), _>);
    Ok(make_gen_body(
        Some(token::Move::default()),
        &block(vec![Stmt::Expr(expr)]),
        &gen_function,
        &quote!(Ok(())),
        &ret_ty,
    ))
}

fn expand_try_stream_block(expr: &mut ExprAsync) -> TokenStream {
    Visitor::new(TryStream).visit_expr_async_mut(expr);

    let gen_function = quote!(::futures_async_stream::__reexport::try_stream::from_generator);
    let ret_ty = quote!(::futures_async_stream::__reexport::result::Result<(), _>);
    make_gen_body(expr.capture, &expr.block, &gen_function, &quote!(Ok(())), &ret_ty)
}
