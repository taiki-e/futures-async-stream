use quote::{quote, quote_spanned};
use syn::{
    parse::Nothing,
    spanned::Spanned,
    visit_mut::{self, VisitMut},
    Expr, ExprAwait, ExprCall, ExprForLoop, ExprYield, Item,
};

use crate::{
    async_stream_block,
    utils::{expr_compile_error, replace_boxed_expr, replace_expr},
};

pub(crate) use Scope::{Closure, Future, Stream};

// =================================================================================================
// Visitor

/// The scope in which `#[for_await]` or `.await` was called.
///
/// The type of generator depends on which scope is called.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Scope {
    /// `async fn`, `async {}`, or `async ||`
    Future,

    /// `#[async_stream]` or `async_stream_block! {}`
    Stream,

    /// `||`, `move ||`, or `static move ||`.
    ///
    /// It cannot call `#[for_await]` or `.await` in this scope.
    Closure,
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
        if let Expr::ForLoop(ExprForLoop { attrs, label, pat, expr: e, body, .. }) = expr {
            // TODO: Should we allow other attributes?
            if !(attrs.len() == 1 && attrs[0].path.is_ident("for_await")) {
                return;
            }
            let attr = attrs.pop().unwrap();
            if let Err(e) = syn::parse2::<Nothing>(attr.tokens) {
                *expr = expr_compile_error(&e);
                return;
            }

            // It needs to adjust the type yielded by the macro because generators used internally by
            // async fn yield `()` type, but generators used internally by `async_stream` yield
            // `Poll<U>` type.
            let match_next = match self.scope {
                Future => {
                    quote! {
                        match ::futures_async_stream::stream::next(&mut __pinned).await {
                            ::futures_async_stream::reexport::option::Option::Some(e) => e,
                            ::futures_async_stream::reexport::option::Option::None => break,
                        }
                    }
                }
                Stream => {
                    quote! {
                        match ::futures_async_stream::stream::poll_next_with_tls_context(
                            ::futures_async_stream::reexport::pin::Pin::as_mut(&mut __pinned),
                        ) {
                            ::futures_async_stream::reexport::task::Poll::Ready(
                                ::futures_async_stream::reexport::option::Option::Some(e),
                            ) => e,
                            ::futures_async_stream::reexport::task::Poll::Ready(
                                ::futures_async_stream::reexport::option::Option::None,
                            ) => break,
                            ::futures_async_stream::reexport::task::Poll::Pending => {
                                yield ::futures_async_stream::reexport::task::Poll::Pending;
                                continue
                            }
                        }
                    }
                }
                Closure => {
                    *expr = expr_compile_error(&error!(
                        &expr,
                        "for await may not be allowed outside of \
                         async blocks, functions, closures, async stream blocks, and functions",
                    ));
                    return;
                }
            };

            *expr = syn::parse_quote! {{
                let mut __pinned = #e;
                let mut __pinned = unsafe {
                    ::futures_async_stream::reexport::pin::Pin::new_unchecked(&mut __pinned)
                };
                #label
                loop {
                    let #pat = #match_next;
                    #body
                }
            }}
        }
    }

    /// Visits `yield <expr>` in `async_stream` scope.
    fn visit_yield(&self, expr: &mut ExprYield) {
        if self.scope != Stream {
            return;
        }

        // Desugar `yield <expr>` into `yield Poll::Ready(<expr>)`.
        replace_boxed_expr(&mut expr.expr, |expr| {
            syn::parse_quote! {
                ::futures_async_stream::reexport::task::Poll::Ready(#expr)
            }
        });
    }

    /// Visits `async_stream_block!` macro.
    fn visit_macro(&self, expr: &mut Expr) {
        replace_expr(expr, |expr| {
            if let Expr::Macro(mut expr) = expr {
                if expr.mac.path.is_ident("async_stream_block") {
                    let mut e: ExprCall =
                        syn::parse(async_stream_block(expr.mac.tokens.into())).unwrap();
                    e.attrs.append(&mut expr.attrs);
                    Expr::Call(e)
                } else {
                    Expr::Macro(expr)
                }
            } else {
                expr
            }
        })
    }

    /// Visits `<base>.await` in `async_stream` scope.
    ///
    /// It needs to adjust the type yielded by the macro because generators used internally by
    /// async fn yield `()` type, but generators used internally by `async_stream` yield
    /// `Poll<U>` type.
    fn visit_await(&self, expr: &mut Expr) {
        if self.scope != Stream {
            return;
        }

        if let Expr::Await(ExprAwait { base, await_token, .. }) = expr {
            *expr = syn::parse2(quote_spanned! { await_token.span() => {
                let mut __pinned = #base;
                loop {
                    if let ::futures_async_stream::reexport::task::Poll::Ready(x) =
                        ::futures_async_stream::stream::poll_with_tls_context(unsafe {
                            ::futures_async_stream::reexport::pin::Pin::new_unchecked(&mut __pinned)
                        })
                    {
                        break x;
                    }

                    yield ::futures_async_stream::reexport::task::Poll::Pending
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
        match &expr {
            Expr::Async(_) => self.scope = Future,
            Expr::Closure(expr) => {
                self.scope = if expr.asyncness.is_some() { Future } else { Closure }
            }
            Expr::Macro(expr) if expr.mac.path.is_ident("async_stream_block") => {
                self.scope = Stream
            }
            _ => {}
        }

        visit_mut::visit_expr_mut(self, expr);
        match expr {
            Expr::Yield(expr) => self.visit_yield(expr),
            Expr::Await(_) => self.visit_await(expr),
            Expr::ForLoop(_) => self.visit_for_loop(expr),
            Expr::Macro(_) => self.visit_macro(expr),
            _ => {}
        }

        // Restore the backup.
        self.scope = tmp;
    }

    fn visit_item_mut(&mut self, _: &mut Item) {
        // Do not recurse into nested items.
    }
}
