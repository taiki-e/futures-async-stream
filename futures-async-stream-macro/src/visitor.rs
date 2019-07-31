use quote::{quote, ToTokens};
use syn::{
    fold::{self, Fold},
    Expr, ExprCall, ExprField, ExprForLoop, ExprMacro, ExprYield, Item, Member,
};

use crate::{async_stream_block, utils::expr_compile_error};

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

    /// Expands `#[for_await] for <pat> in <expr> { .. }`.
    fn expand_for_loop(&self, mut expr: ExprForLoop) -> Expr {
        // TODO: Should we allow other attributes?
        if !(expr.attrs.len() == 1 && expr.attrs[0].path.is_ident("for_await")) {
            return Expr::ForLoop(expr);
        }
        if !expr.attrs[0].tts.is_empty() {
            return expr_compile_error(&error!(
                expr.attrs.pop(),
                "attribute must be of the form `#[for_await]`"
            ));
        }

        // It needs to adjust the type yielded by the macro because generators used internally by
        // async fn yield `()` type, but generators used internally by `async_stream` yield
        // `Poll<U>` type.
        let match_next = match self.scope {
            Future => {
                quote! {
                    match ::futures_async_stream::stream::next(&mut __pinned).await {
                        ::futures_async_stream::core_reexport::option::Option::Some(e) => e,
                        ::futures_async_stream::core_reexport::option::Option::None => break,
                    }
                }
            }
            Stream => {
                quote! {
                    match ::futures_async_stream::stream::poll_next_with_tls_context(
                        ::futures_async_stream::core_reexport::pin::Pin::as_mut(&mut __pinned),
                    ) {
                        ::futures_async_stream::core_reexport::task::Poll::Ready(
                            ::futures_async_stream::core_reexport::option::Option::Some(e),
                        ) => e,
                        ::futures_async_stream::core_reexport::task::Poll::Ready(
                            ::futures_async_stream::core_reexport::option::Option::None,
                        ) => break,
                        ::futures_async_stream::core_reexport::task::Poll::Pending => {
                            yield ::futures_async_stream::core_reexport::task::Poll::Pending;
                            continue
                        }
                    }
                }
            }
            Closure => {
                return expr_compile_error(&error!(
                    expr,
                    "#[for_await] cannot be allowed outside of \
                     async closures, blocks, functions, async_stream blocks, and functions.",
                ))
            }
        };

        let ExprForLoop { label, pat, expr, body, .. } = &expr;
        syn::parse_quote! {{
            let mut __pinned = #expr;
            let mut __pinned = unsafe {
                ::futures_async_stream::core_reexport::pin::Pin::new_unchecked(&mut __pinned)
            };
            #label
            loop {
                let #pat = #match_next;
                #body
            }
        }}
    }

    /// Expands `yield <expr>` in `async_stream` scope.
    fn expand_yield(&self, expr: ExprYield) -> ExprYield {
        if self.scope != Stream {
            return expr;
        }

        // Desugar `yield <expr>` into `yield Poll::Ready(<expr>)`.
        let ExprYield { attrs, yield_token, expr } = expr;
        let expr = expr.map_or_else(|| quote!(()), ToTokens::into_token_stream);
        let expr = syn::parse_quote! {
            ::futures_async_stream::core_reexport::task::Poll::Ready(#expr)
        };
        ExprYield { attrs, yield_token, expr: Some(Box::new(expr)) }
    }

    /// Expands `async_stream_block!` macro.
    fn expand_macro(&mut self, mut expr: ExprMacro) -> Expr {
        if expr.mac.path.is_ident("async_stream_block") {
            let mut e: ExprCall = syn::parse(async_stream_block(expr.mac.tts.into())).unwrap();
            e.attrs.append(&mut expr.attrs);
            Expr::Call(e)
        } else {
            Expr::Macro(expr)
        }
    }

    /// Expands `<expr>.await` in `async_stream` scope.
    ///
    /// It needs to adjust the type yielded by the macro because generators used internally by
    /// async fn yield `()` type, but generators used internally by `async_stream` yield
    /// `Poll<U>` type.
    fn expand_await(&mut self, expr: ExprField) -> Expr {
        if self.scope != Stream {
            return Expr::Field(expr);
        }

        match &expr.member {
            Member::Named(x) if x == "await" => {}
            _ => return Expr::Field(expr),
        }
        let expr = expr.base;

        syn::parse2(quote! {{
            let mut __pinned = #expr;
            loop {
                if let ::futures_async_stream::core_reexport::task::Poll::Ready(x) =
                    ::futures_async_stream::stream::poll_with_tls_context(unsafe {
                        ::futures_async_stream::core_reexport::pin::Pin::new_unchecked(&mut __pinned)
                    })
                {
                    break x;
                }

                yield ::futures_async_stream::core_reexport::task::Poll::Pending
            }
        }})
        // As macro input (<expr>) is untrusted, use `syn::parse2` + `expr_compile_error`
        // instead of `syn::parse_quote!` to generate better error messages (`syn::parse_quote!`
        // panics if fail to parse).
        .unwrap_or_else(|e| expr_compile_error(&e))
    }
}

impl Fold for Visitor {
    fn fold_expr(&mut self, expr: Expr) -> Expr {
        // Backup current scope and adjust the scope. This must be done before folding expr.
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

        let expr = match fold::fold_expr(self, expr) {
            Expr::Yield(expr) => Expr::Yield(self.expand_yield(expr)),
            Expr::Field(expr) => self.expand_await(expr),
            Expr::ForLoop(expr) => self.expand_for_loop(expr),
            Expr::Macro(expr) => self.expand_macro(expr),
            expr => expr,
        };

        // Restore the backup.
        self.scope = tmp;
        expr
    }

    // Stop at item bounds
    fn fold_item(&mut self, item: Item) -> Item {
        item
    }
}
