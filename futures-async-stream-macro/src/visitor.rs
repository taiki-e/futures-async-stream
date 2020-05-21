use quote::{quote, quote_spanned};
use syn::{
    spanned::Spanned,
    visit_mut::{self, VisitMut},
    Expr, ExprAwait, ExprCall, ExprForLoop, ExprYield, Item,
};

use crate::{
    parse, stream, stream_block, try_stream_block,
    utils::{expr_compile_error, replace_expr, unit, SliceExt, TASK_CONTEXT},
};

/// The scope in which `#[for_await]`, `.await`, or `yield` was called.
///
/// The type of generator depends on which scope is called.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Scope {
    /// `async fn`, `async {}`, or `async ||`
    Future,

    /// `#[stream]` (this)
    Stream,

    /// `#[try_stream]` (this)
    TryStream,

    /// `||`, `move ||`, or `static move ||`.
    ///
    /// It cannot call `#[for_await]` or `.await` in this scope.
    Closure,

    /// `#[stream]` or `#[try_stream]` (other)
    Other,
}

impl Scope {
    fn is_stream(self) -> bool {
        match self {
            Self::Stream | Self::TryStream => true,
            _ => false,
        }
    }
}

impl Default for Scope {
    fn default() -> Self {
        Self::Future
    }
}

#[derive(Default)]
pub(crate) struct Visitor {
    scope: Scope,
}

impl Visitor {
    pub(crate) fn new(scope: Scope) -> Self {
        Self { scope }
    }

    /// Visits `#[for_await] for <pat> in <expr> { .. }`.
    pub(crate) fn visit_for_loop(&self, expr: &mut Expr) {
        // Desugar
        // from:
        //
        // #[for_await]
        // <label> for <pat> in <e> {
        //     <body.stmts>
        // }
        //
        // into:
        //
        // {
        //     let mut __pinned = <e>;
        //     let mut __pinned = unsafe { Pin::new_unchecked(&mut __pinned) };
        //     <label> loop {
        //         let <pat> = <match_next>;
        //         <body.stmts>
        //     }
        // }
        //
        if let Expr::ForLoop(ExprForLoop { attrs, label, pat, expr: e, body, .. }) = expr {
            match attrs.position_exact("for_await") {
                Err(e) => {
                    *expr = expr_compile_error(&e);
                    return;
                }
                Ok(None) => return,
                Ok(Some(i)) => {
                    attrs.remove(i);
                }
            }

            let pinned = def_site_ident!("__pinned");

            // It needs to adjust the type yielded by the macro because generators used internally by
            // async fn yield `()` type, but generators used internally by `stream` yield
            // `Poll<U>` type.
            let match_next = match self.scope {
                Scope::Future => {
                    quote! {
                        match ::futures_async_stream::__reexport::stream::next(&mut #pinned).await {
                            ::futures_async_stream::__reexport::option::Option::Some(e) => e,
                            ::futures_async_stream::__reexport::option::Option::None => break,
                        }
                    }
                }
                Scope::Stream | Scope::TryStream => {
                    let task_context = def_site_ident!(TASK_CONTEXT);
                    quote! {
                        match unsafe {
                            ::futures_async_stream::__reexport::stream::Stream::poll_next(
                                ::futures_async_stream::__reexport::pin::Pin::as_mut(&mut #pinned),
                                ::futures_async_stream::__reexport::future::get_context(#task_context),
                            )
                        } {
                            ::futures_async_stream::__reexport::task::Poll::Ready(
                                ::futures_async_stream::__reexport::option::Option::Some(e),
                            ) => e,
                            ::futures_async_stream::__reexport::task::Poll::Ready(
                                ::futures_async_stream::__reexport::option::Option::None,
                            ) => break,
                            ::futures_async_stream::__reexport::task::Poll::Pending => {
                                #task_context = yield ::futures_async_stream::__reexport::task::Poll::Pending;
                                continue;
                            }
                        }
                    }
                }
                Scope::Closure => {
                    *expr = expr_compile_error(&error!(
                        &expr,
                        "for await may not be allowed outside of \
                         async blocks, functions, closures, async stream blocks, and functions",
                    ));
                    return;
                }
                Scope::Other => unreachable!(),
            };

            body.stmts.insert(0, syn::parse_quote!(let #pat = #match_next;));
            *expr = syn::parse_quote! {{
                let mut #pinned = #e;
                let mut #pinned = unsafe {
                    ::futures_async_stream::__reexport::pin::Pin::new_unchecked(&mut #pinned)
                };
                #label loop #body
            }}
        }
    }

    /// Visits `yield <expr>`.
    fn visit_yield(&self, expr: &mut Expr) {
        if !self.scope.is_stream() {
            return;
        }

        // Desugar `yield <e>` into `task_context = yield Poll::Ready(<e>)`.
        if let Expr::Yield(ExprYield { yield_token, expr: e, .. }) = expr {
            if e.is_none() {
                e.replace(Box::new(unit()));
            }

            let task_context = def_site_ident!(TASK_CONTEXT);
            *expr = syn::parse_quote! {
                #task_context = #yield_token ::futures_async_stream::__reexport::task::Poll::Ready(#e)
            };
        }
    }

    /// Visits `stream_block!` macro.
    fn visit_macro(&self, expr: &mut Expr) {
        if self.scope != Scope::Other {
            return;
        }

        replace_expr(expr, |expr| {
            if let Expr::Macro(mut expr) = expr {
                let mut e: ExprCall = if expr.mac.path.is_ident("stream_block") {
                    syn::parse(stream_block(expr.mac.tokens.into())).unwrap()
                } else if expr.mac.path.is_ident("try_stream_block") {
                    syn::parse(try_stream_block(expr.mac.tokens.into())).unwrap()
                } else {
                    return Expr::Macro(expr);
                };
                e.attrs.append(&mut expr.attrs);
                Expr::Call(e)
            } else {
                unreachable!()
            }
        })
    }

    /// Visits `#[stream] async (move) <block>`.
    fn visit_async(&self, expr: &mut Expr) {
        if self.scope != Scope::Other {
            return;
        }

        if let Expr::Async(e) = expr {
            match (e.attrs.position_exact("stream"), e.attrs.position_exact("try_stream")) {
                (Err(e), _) | (_, Err(e)) => {
                    *expr = expr_compile_error(&e);
                }
                (Ok(Some(_)), Ok(Some(i))) => {
                    *expr = expr_compile_error(&error!(
                        e.attrs.remove(i),
                        "#[stream] and #[try_stream] may not be used at the same time"
                    ));
                }
                (Ok(Some(i)), _) => {
                    e.attrs.remove(i);
                    *expr = match stream::parse_async(e, parse::Context::Stream) {
                        Ok(tokens) => syn::parse2(tokens).unwrap(),
                        Err(e) => expr_compile_error(&e),
                    }
                }
                (_, Ok(Some(i))) => {
                    e.attrs.remove(i);
                    *expr = match stream::parse_async(e, parse::Context::TryStream) {
                        Ok(tokens) => syn::parse2(tokens).unwrap(),
                        Err(e) => expr_compile_error(&e),
                    }
                }
                (Ok(None), Ok(None)) => unreachable!(),
            }
        }
    }

    /// Visits `<base>.await`.
    ///
    /// It needs to adjust the type yielded by the macro because generators used internally by
    /// async fn yield `()` type, but generators used internally by `stream` yield
    /// `Poll<U>` type.
    fn visit_await(&self, expr: &mut Expr) {
        if !self.scope.is_stream() {
            return;
        }

        // Desugar `<base>.await` into:
        //
        // {
        //     let mut __pinned = <base>;
        //     let mut __pinned = unsafe { Pin::new_unchecked(&mut __pinned) };
        //     loop {
        //         if let Poll::Ready(result) = unsafe {
        //             Future::poll(Pin::as_mut(&mut __pinned), get_context(task_context))
        //         } {
        //             break result;
        //         }
        //         task_context = yield Poll::Pending;
        //     }
        // }
        if let Expr::Await(ExprAwait { base, await_token, .. }) = expr {
            let task_context = def_site_ident!(TASK_CONTEXT);
            *expr = syn::parse2(quote_spanned! { await_token.span() => {
                let mut __pinned = #base;
                let mut __pinned = unsafe { ::futures_async_stream::__reexport::pin::Pin::new_unchecked(&mut __pinned) };
                loop {
                    if let ::futures_async_stream::__reexport::task::Poll::Ready(result) = unsafe {
                        ::futures_async_stream::__reexport::future::Future::poll(
                            ::futures_async_stream::__reexport::pin::Pin::as_mut(&mut __pinned),
                            ::futures_async_stream::__reexport::future::get_context(#task_context),
                        )
                    } {
                        break result;
                    }
                    #task_context = yield ::futures_async_stream::__reexport::task::Poll::Pending;
                }
            }})
            .unwrap();
        }
    }
}

impl VisitMut for Visitor {
    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        // Backup current scope and adjust the scope. This must be done before visiting expr.
        let tmp = self.scope;
        match expr {
            Expr::Async(expr)
                if expr.attrs.iter().any(|attr| {
                    attr.path.is_ident("stream") || attr.path.is_ident("try_stream")
                }) =>
            {
                self.scope = Scope::Other
            }
            Expr::Async(_) => self.scope = Scope::Future,
            Expr::Closure(expr) => {
                self.scope = if expr.asyncness.is_some() { Scope::Future } else { Scope::Closure }
            }
            Expr::Macro(expr)
                if expr.mac.path.is_ident("stream_block")
                    || expr.mac.path.is_ident("try_stream_block") =>
            {
                self.scope = Scope::Other
            }
            _ => {}
        }

        if self.scope != Scope::Other {
            visit_mut::visit_expr_mut(self, expr);
        }
        match expr {
            Expr::Async(_) => self.visit_async(expr),
            Expr::Await(_) => self.visit_await(expr),
            Expr::ForLoop(_) => self.visit_for_loop(expr),
            Expr::Macro(_) => self.visit_macro(expr),
            Expr::Yield(_) => self.visit_yield(expr),
            _ => {}
        }

        // Restore the backup.
        self.scope = tmp;
    }

    fn visit_item_mut(&mut self, _: &mut Item) {
        // Do not recurse into nested items.
    }
}
