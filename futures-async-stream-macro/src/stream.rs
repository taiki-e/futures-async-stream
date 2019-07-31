use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{
    fold::Fold,
    parse::{Parse, ParseStream},
    token, ArgCaptured, Expr, FnArg, FnDecl, Ident, ItemFn, Pat, PatIdent, Result, ReturnType,
    Token, Type, TypeTuple,
};

use crate::{
    elision,
    utils::{first_last, respan},
    visitor::{Stream, Visitor},
};

// =================================================================================================
// async_stream

pub(super) fn async_stream(args: TokenStream, input: TokenStream) -> Result<TokenStream> {
    parse_async_stream_fn(args, input)
}

struct Arg(Type);

mod kw {
    syn::custom_keyword!(item);
}

impl Parse for Arg {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let _: kw::item = input.parse()?;
        let _: Token![=] = input.parse()?;
        input.parse().map(Self)
    }
}

fn parse_async_stream_fn(args: TokenStream, input: TokenStream) -> Result<TokenStream> {
    let args: Arg = syn::parse2(args)?;
    let item: ItemFn = syn::parse2(input)?;

    if let Some(constness) = item.constness {
        return Err(error!(constness, "async stream may not be const"));
    }
    if let Some(variadic) = item.decl.variadic {
        return Err(error!(variadic, "async stream may not be variadic"));
    }

    if item.asyncness.is_none() {
        return Err(error!(item.decl.fn_token, "async stream must be declared as async"));
    }

    if let ReturnType::Type(_, ty) = &item.decl.output {
        match &**ty {
            Type::Tuple(TypeTuple { elems, .. }) if elems.is_empty() => {}
            _ => return Err(error!(ty, "async stream must return the unit type")),
        }
    }

    Ok(expand_async_stream_fn(item, &args.0))
}

fn expand_async_stream_fn(item: ItemFn, item_ty: &Type) -> TokenStream {
    let ItemFn { ident, vis, unsafety, abi, block, decl, attrs, .. } = item;
    let FnDecl { inputs, mut generics, fn_token, .. } = *decl;
    let where_clause = &generics.where_clause;

    // Desugar `async fn`
    // from:
    //
    //      async fn foo(ref a: u32) -> u32 {
    //          // ...
    //      }
    //
    // into:
    //
    //      fn foo(__arg_0: u32) -> impl Stream<Item = u32> {
    //          from_generator(static move || {
    //              let ref a = __arg_0;
    //
    //              // ...
    //          })
    //      }
    //
    // We notably skip everything related to `self` which typically doesn't have
    // many patterns with it and just gets captured naturally.
    let mut inputs_no_patterns = Vec::new();
    let mut patterns = Vec::new();
    let mut temp_bindings = Vec::new();
    for (i, input) in inputs.into_iter().enumerate() {
        match input {
            FnArg::Captured(ArgCaptured { pat: Pat::Ident(ref pat), .. })
                if pat.ident == "self" =>
            {
                // `self: Box<Self>` will get captured naturally
                inputs_no_patterns.push(input);
            }
            FnArg::Captured(ArgCaptured {
                pat: pat @ Pat::Ident(PatIdent { by_ref: Some(_), .. }),
                ty,
                colon_token,
            }) => {
                // `ref a: B` (or some similar pattern)
                patterns.push(pat);
                let ident = Ident::new(&format!("__arg_{}", i), Span::call_site());
                temp_bindings.push(ident.clone());
                let pat = PatIdent { by_ref: None, mutability: None, ident, subpat: None }.into();
                inputs_no_patterns.push(ArgCaptured { pat, ty, colon_token }.into());
            }
            _ => {
                // Other arguments get captured naturally
                inputs_no_patterns.push(input);
            }
        }
    }

    // Expand `#[for_await]` and `.await`.
    let block = Visitor::new(Stream).fold_block(*block);

    let block_inner = quote! {
        #( let #patterns = #temp_bindings; )*
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
            return;
            loop { yield ::futures_async_stream::core_reexport::task::Poll::Pending }
        }
    };
    let mut gen_body = TokenStream::new();
    block.brace_token.surround(&mut gen_body, |tokens| {
        gen_body_inner.to_tokens(tokens);
    });

    // Give the invocation of the `from_generator` function the same span as the `item_ty`
    // as currently errors related to it being a result are targeted here. Not
    // sure if more errors will highlight this function call...
    let output_span = first_last(item_ty);
    let gen_function = quote!(::futures_async_stream::stream::from_generator);
    let gen_function = respan(gen_function, output_span);
    let body_inner = quote! {
        #gen_function (static move || -> () #gen_body)
    };
    let mut body = TokenStream::new();
    block.brace_token.surround(&mut body, |tokens| {
        body_inner.to_tokens(tokens);
    });

    let inputs_no_patterns = elision::unelide_lifetimes(&mut generics.params, inputs_no_patterns);
    let lifetimes = generics.lifetimes().map(|l| &l.lifetime);

    // Raw `impl` breaks syntax highlighting in some editors.
    let impl_token = token::Impl::default();
    let return_ty = quote! {
        #impl_token ::futures_async_stream::stream::Stream<Item = #item_ty> + #(#lifetimes +)*
    };
    let return_ty = respan(return_ty, output_span);
    quote! {
        #(#attrs)*
        #vis #unsafety #abi
        #fn_token #ident #generics (#(#inputs_no_patterns),*)
            -> #return_ty
            #where_clause
        #body
    }
}

// =================================================================================================
// async_stream_block

pub(super) fn async_stream_block(input: TokenStream) -> Result<TokenStream> {
    syn::parse2(input).map(expand_async_stream_block)
}

fn expand_async_stream_block(expr: Expr) -> TokenStream {
    let expr = Visitor::new(Stream).fold_expr(expr);

    let gen_body = quote! {{
        let (): () = #expr;

        // Ensure that this closure is a generator, even if it doesn't
        // have any `yield` statements.
        #[allow(unreachable_code)]
        {
            return;
            loop { yield ::futures_async_stream::core_reexport::task::Poll::Pending }
        }
    }};

    let gen_function = quote!(::futures_async_stream::stream::from_generator);
    quote! {
        #gen_function (static move || -> () #gen_body)
    }
}
