use proc_macro2::TokenStream;
use syn::{
    parse::{Parse, ParseStream},
    token, Abi, Attribute, Block, ExprAsync, Result, ReturnType, Signature, Token, TraitItemMethod,
    Type, Visibility,
};

use crate::{utils::SliceExt, visitor::Scope};

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

pub(crate) fn parse(input: TokenStream, cx: Context) -> Result<FnOrAsync> {
    let input = syn::parse2(input)?;
    match &input {
        FnOrAsync::Fn(sig) => {
            validate_signature(Some(sig), &sig.attrs, cx)?;
            Ok(input)
        }
        FnOrAsync::Async(expr, _) => {
            validate_signature(None, &expr.attrs, cx)?;
            Ok(input)
        }
        FnOrAsync::NotAsync => bail!(
            // Highlight the attribute itself, like `derive` and `proc_macro` do.
            TokenStream::new(),
            "#[{}] attribute may only be used on async functions or async blocks",
            cx.as_str()
        ),
    }
}

// Based on https://github.com/dtolnay/syn/blob/1.0.30/src/item.rs#L1625-L1632
fn peek_signature(input: ParseStream<'_>) -> bool {
    let fork = input.fork();
    fork.parse::<Visibility>().is_ok()
        && fork.parse::<Option<Token![const]>>().is_ok()
        && fork.parse::<Option<Token![async]>>().is_ok()
        && fork.parse::<Option<Token![unsafe]>>().is_ok()
        && fork.parse::<Option<Abi>>().is_ok()
        && fork.peek(Token![fn])
}

fn validate_signature(item: Option<&FnSig>, attrs: &[Attribute], cx: Context) -> Result<()> {
    if let Some(item) = item {
        if item.sig.asyncness.is_none() {
            bail!(item.sig.fn_token, "async stream must be declared as async");
        }
        if let Some(constness) = item.sig.constness {
            bail!(constness, "async stream may not be const");
        }
        if let Some(variadic) = &item.sig.variadic {
            bail!(variadic, "async stream may not be variadic");
        }

        if let ReturnType::Type(_, ty) = &item.sig.output {
            match &**ty {
                Type::Tuple(ty) if ty.elems.is_empty() => {}
                _ => bail!(ty, "async stream must return the unit type"),
            }
        }
    }

    let (duplicate, another) = match cx {
        Context::Stream => ("stream", "try_stream"),
        Context::TryStream => ("try_stream", "stream"),
    };
    if let Some(attr) = attrs.find(duplicate) {
        bail!(attr, "duplicate #[{}] attribute", duplicate)
    } else if let Some(attr) = attrs.find(another) {
        bail!(attr, "#[stream] and #[try_stream] may not be used at the same time")
    }
    Ok(())
}

#[allow(clippy::large_enum_variant)]
pub(crate) enum FnOrAsync {
    Fn(FnSig),
    Async(ExprAsync, Option<Token![;]>),
    NotAsync,
}

impl Parse for FnOrAsync {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut attrs = input.call(Attribute::parse_outer)?;

        if peek_signature(input) {
            let vis: Visibility = input.parse()?;
            let method: TraitItemMethod = input.parse()?;

            let mut fn_sig: FnSig = method.into();
            attrs.append(&mut fn_sig.attrs);
            fn_sig.attrs = attrs;
            fn_sig.vis = vis;

            Ok(Self::Fn(fn_sig))
        } else if input.peek(Token![async]) {
            let mut expr: ExprAsync = input.parse()?;
            attrs.append(&mut expr.attrs);
            expr.attrs = attrs;

            Ok(Self::Async(expr, input.parse()?))
        } else {
            input.parse::<TokenStream>()?; // ignore all inputs
            Ok(Self::NotAsync)
        }
    }
}

pub(crate) struct FnSig {
    pub(crate) attrs: Vec<Attribute>,
    pub(crate) vis: Visibility,
    pub(crate) sig: Signature,
    pub(crate) block: Block,
    pub(crate) semi: Option<Token![;]>,
}

impl From<TraitItemMethod> for FnSig {
    fn from(item: TraitItemMethod) -> Self {
        if let Some(block) = item.default {
            Self { attrs: item.attrs, vis: Visibility::Inherited, sig: item.sig, block, semi: None }
        } else {
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
