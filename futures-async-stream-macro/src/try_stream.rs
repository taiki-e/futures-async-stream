use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    visit_mut::VisitMut,
    *,
};

use crate::{
    elision,
    stream::{expand_async_body, make_gen_body, validate_async_stream_fn, FnSig, ReturnTypeKind},
    utils::{block, first_last, respan},
    visitor::{TryStream, Visitor},
};

// =================================================================================================
// async_try_stream

pub(super) fn attribute(args: TokenStream, input: TokenStream) -> Result<TokenStream> {
    parse_async_try_stream_fn(args, input)
}

mod kw {
    syn::custom_keyword!(ok);
    syn::custom_keyword!(error);
}

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

struct Args {
    ok: Type,
    error: Type,
    boxed: ReturnTypeKind,
}

impl Parse for Args {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
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

fn parse_async_try_stream_fn(args: TokenStream, input: TokenStream) -> Result<TokenStream> {
    let args: Args = syn::parse2(args)?;
    let item = FnSig::parse(input, args.boxed)?;

    validate_async_stream_fn(&item)?;
    Ok(expand_async_try_stream_fn(item, &args))
}

fn expand_async_try_stream_fn(item: FnSig, args: &Args) -> TokenStream {
    let FnSig { attrs, vis, sig, mut block, semi } = item;
    let Signature { unsafety, abi, fn_token, ident, mut generics, inputs, .. } = sig;
    let where_clause = &generics.where_clause;

    let (mut arguments, statements) = expand_async_body(inputs);

    // Visit `#[for_await]` and `.await`.
    Visitor::new(TryStream).visit_block_mut(&mut block);

    let ok = &args.ok;
    let error = &args.error;

    // Give the invocation of the `from_generator` function the same span as the `item_ty`
    // as currently errors related to it being a result are targeted here. Not
    // sure if more errors will highlight this function call...
    let output_span = first_last(ok);
    let gen_function = quote!(::futures_async_stream::try_stream::from_generator);
    let gen_function = respan(gen_function, output_span);
    let ret_ty = quote!(::futures_async_stream::reexport::result::Result<(), #error>);
    let mut body_inner =
        make_gen_body(&statements, &block, &gen_function, &quote!(Ok(())), &ret_ty);

    if let ReturnTypeKind::Boxed { .. } = args.boxed {
        let body = quote! { ::futures_async_stream::reexport::boxed::Box::pin(#body_inner) };
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
                #impl_token ::futures_async_stream::reexport::Stream<
                    Item = ::futures_async_stream::reexport::result::Result<#ok, #error>
                > + #(#lifetimes +)*
            }
        }
        ReturnTypeKind::Boxed { send } => {
            let send = if send { Some(quote!(+ Send)) } else { None };
            quote! {
                ::futures_async_stream::reexport::pin::Pin<
                    ::futures_async_stream::reexport::boxed::Box<
                        dyn ::futures_async_stream::reexport::Stream<
                            Item = ::futures_async_stream::reexport::result::Result<#ok, #error>
                        > #send + #(#lifetimes +)*
                    >
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
// async_try_stream_block

pub(super) fn block_macro(input: TokenStream) -> Result<TokenStream> {
    syn::parse2(input).map(expand_async_try_stream_block)
}

fn expand_async_try_stream_block(mut expr: Expr) -> TokenStream {
    Visitor::new(TryStream).visit_expr_mut(&mut expr);

    let gen_function = quote!(::futures_async_stream::try_stream::from_generator);
    let ret_ty = quote!(::futures_async_stream::reexport::result::Result<(), _>);
    make_gen_body(&[], &block(vec![Stmt::Expr(expr)]), &gen_function, &quote!(Ok(())), &ret_ty)
}
