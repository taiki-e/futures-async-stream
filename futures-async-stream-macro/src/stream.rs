use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    token,
    visit_mut::VisitMut,
    ArgCaptured, Expr, FnArg, FnDecl, Ident, ItemFn, Pat, PatIdent, Result, ReturnType, Token,
    Type, TypeTuple, *,
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

struct Args {
    item: Type,
    boxed: ReturnTypeKind,
}

// TODO: rename to `ReturnType`
#[derive(Clone, Copy)]
enum ReturnTypeKind {
    // impl Stream<Item = ..> $(+ $lifetime)?
    Default,
    // Pin<Box<dyn Stream<Item = ..> (+ Send)? $(+ $lifetime)?>>
    Boxed { send: bool },
}

impl Parse for Args {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut item = None;
        let mut boxed = ReturnTypeKind::Default;
        while !input.is_empty() {
            if input.peek(kw::boxed) {
                let i: kw::boxed = input.parse()?;
                match boxed {
                    ReturnTypeKind::Default => boxed = ReturnTypeKind::Boxed { send: true },
                    ReturnTypeKind::Boxed { send: true } => {
                        return Err(error!(i, "duplicate `boxed` argument"))
                    }
                    ReturnTypeKind::Boxed { send: false } => {
                        return Err(error!(
                            i,
                            "`boxed` and `boxed_local` cannot be used at the same time."
                        ))
                    }
                }
            } else if input.peek(kw::boxed_local) {
                let i: kw::boxed_local = input.parse()?;
                match boxed {
                    ReturnTypeKind::Default => boxed = ReturnTypeKind::Boxed { send: false },
                    ReturnTypeKind::Boxed { send: false } => {
                        return Err(error!(i, "duplicate `boxed_local` argument"))
                    }
                    ReturnTypeKind::Boxed { send: true } => {
                        return Err(error!(
                            i,
                            "`boxed` and `boxed_local` cannot be used at the same time."
                        ))
                    }
                }
            } else {
                let i: Item = input.parse()?;
                if item.is_some() {
                    return Err(error!(i.0, "duplicate `item` argument"));
                }
                item = Some(i.0);
            }
            let _: Option<Token![,]> = input.parse()?;
        }

        if let Some(item) = item {
            Ok(Self { item, boxed })
        } else {
            let _: kw::item = input.parse()?;
            unreachable!()
        }
    }
}

struct FnSig {
    attrs: Vec<Attribute>,
    vis: Visibility,
    sig: MethodSig,
    block: Block,
    semi: Option<Token![;]>,
}

impl From<ItemFn> for FnSig {
    fn from(item: ItemFn) -> Self {
        Self {
            attrs: item.attrs,
            vis: item.vis,
            sig: MethodSig {
                constness: item.constness,
                asyncness: item.asyncness,
                unsafety: item.unsafety,
                abi: item.abi,
                ident: item.ident,
                decl: *item.decl,
            },
            block: *item.block,
            semi: None,
        }
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

fn parse_async_stream_fn(args: TokenStream, input: TokenStream) -> Result<TokenStream> {
    let args: Args = syn::parse2(args)?;
    let item: FnSig = {
        let input2 = input.clone();
        match syn::parse2(input) {
            Ok(item) => ItemFn::into(item),
            Err(e) => match syn::parse2(input2) {
                Ok(item) => TraitItemMethod::into(item),
                Err(_e) => return Err(e),
            },
        }
    };

    if let Some(constness) = item.sig.constness {
        return Err(error!(constness, "async stream may not be const"));
    }
    if let Some(variadic) = item.sig.decl.variadic {
        return Err(error!(variadic, "async stream may not be variadic"));
    }

    if item.sig.asyncness.is_none() {
        return Err(error!(item.sig.decl.fn_token, "async stream must be declared as async"));
    }

    if let ReturnType::Type(_, ty) = &item.sig.decl.output {
        match &**ty {
            Type::Tuple(TypeTuple { elems, .. }) if elems.is_empty() => {}
            _ => return Err(error!(ty, "async stream must return the unit type")),
        }
    }

    Ok(expand_async_stream_fn(item, &args))
}

fn expand_async_stream_fn(item: FnSig, args: &Args) -> TokenStream {
    let FnSig { attrs, vis, sig, mut block, semi } = item;
    let MethodSig { unsafety, abi, ident, decl, .. } = sig;
    let FnDecl { inputs, mut generics, fn_token, .. } = decl;
    let where_clause = &generics.where_clause;

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

    // Visit `#[for_await]` and `.await`.
    Visitor::new(Stream).visit_block_mut(&mut block);

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

    let item = &args.item;

    // Give the invocation of the `from_generator` function the same span as the `item`
    // as currently errors related to it being a result are targeted here. Not
    // sure if more errors will highlight this function call...
    let output_span = first_last(item);
    let gen_function = quote!(::futures_async_stream::stream::from_generator);
    let gen_function = respan(gen_function, output_span);
    let mut body_inner = quote! {
        #gen_function (static move || -> () #gen_body)
    };
    if let ReturnTypeKind::Boxed { .. } = args.boxed {
        let body = quote! { ::futures_async_stream::alloc_reexport::boxed::Box::pin(#body_inner) };
        body_inner = respan(body, output_span);
    }

    let mut body = TokenStream::new();
    block.brace_token.surround(&mut body, |tokens| {
        body_inner.to_tokens(tokens);
    });

    elision::unelide_lifetimes(&mut generics.params, &mut inputs_no_patterns);
    let lifetimes = generics.lifetimes().map(|l| &l.lifetime);

    let return_ty = match args.boxed {
        ReturnTypeKind::Default => {
            // Raw `impl` breaks syntax highlighting in some editors.
            let impl_token = token::Impl::default();
            quote! {
                #impl_token ::futures_async_stream::stream::Stream<Item = #item> + #(#lifetimes +)*
            }
        }
        ReturnTypeKind::Boxed { send } => {
            let send = if send { Some(quote!(+ Send)) } else { None };
            quote! {
                ::futures_async_stream::core_reexport::pin::Pin<
                    ::futures_async_stream::alloc_reexport::boxed::Box<
                        dyn ::futures_async_stream::stream::Stream<Item = #item> #send + #(#lifetimes +)*
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

fn expand_async_stream_block(mut expr: Expr) -> TokenStream {
    Visitor::new(Stream).visit_expr_mut(&mut expr);

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
