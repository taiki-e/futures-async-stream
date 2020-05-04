use quote::{quote, quote_spanned};
use syn::{
    spanned::Spanned,
    visit_mut::{self, VisitMut},
    Expr, ExprAwait, ExprCall, ExprForLoop, ExprYield, Item,
};

use crate::{
    async_stream_block, async_try_stream_block,
    utils::{expr_compile_error, parse_as_empty, replace_expr, unit},
};

use Scope::Skip;
pub(crate) use Scope::{Closure, Future, Stream, TryStream};

// =================================================================================================
// Visitor

/// The scope in which `#[for_await]` or `.await` was called.
///
/// The type of generator depends on which scope is called.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Scope {
    /// `async fn`, `async {}`, or `async ||`
    Future,

    /// `#[async_stream]`
    Stream,

    /// `#[async_try_stream]`
    TryStream,

    /// `||`, `move ||`, or `static move ||`.
    ///
    /// It cannot call `#[for_await]` or `.await` in this scope.
    Closure,

    Skip,
}

impl Scope {
    fn is_stream(self) -> bool {
        match self {
            Stream | TryStream => true,
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
            // TODO: Should we allow other attributes?
            if !(attrs.len() == 1 && attrs[0].path.is_ident("for_await")) {
                return;
            }
            let attr = attrs.pop().unwrap();
            if let Err(e) = parse_as_empty(&attr.tokens) {
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
                            ::futures_async_stream::__reexport::option::Option::Some(e) => e,
                            ::futures_async_stream::__reexport::option::Option::None => break,
                        }
                    }
                }
                Stream | TryStream => {
                    quote! {
                        match unsafe { ::futures_async_stream::stream::Stream::poll_next(
                            ::futures_async_stream::__reexport::pin::Pin::as_mut(&mut __pinned),
                            ::futures_async_stream::future::get_context(__task_context),
                        ) } {
                            ::futures_async_stream::__reexport::task::Poll::Ready(
                                ::futures_async_stream::__reexport::option::Option::Some(e),
                            ) => e,
                            ::futures_async_stream::__reexport::task::Poll::Ready(
                                ::futures_async_stream::__reexport::option::Option::None,
                            ) => break,
                            ::futures_async_stream::__reexport::task::Poll::Pending => {
                                __task_context = yield ::futures_async_stream::__reexport::task::Poll::Pending;
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
                Skip => unreachable!(),
            };

            body.stmts.insert(0, syn::parse_quote! { let #pat = #match_next; });
            *expr = syn::parse_quote! {{
                let mut __pinned = #e;
                let mut __pinned = unsafe {
                    ::futures_async_stream::__reexport::pin::Pin::new_unchecked(&mut __pinned)
                };
                #label loop #body
            }}
        }
    }

    /// Visits `yield <expr>` in `stream` scope.
    fn visit_yield(&self, expr: &mut Expr) {
        if !self.scope.is_stream() {
            return;
        }

        // Desugar `yield <expr>` into `__task_context = yield Poll::Ready(<expr>)`.
        if let Expr::Yield(ExprYield { yield_token, expr: e, .. }) = expr {
            if e.is_none() {
                e.replace(Box::new(unit()));
            }

            *expr = syn::parse_quote!(
                __task_context = #yield_token ::futures_async_stream::__reexport::task::Poll::Ready(#e)
            );
        }
    }

    /// Visits `async_stream_block!` macro.
    fn visit_macro(&self, expr: &mut Expr) {
        if !self.scope.is_stream() {
            return;
        }

        replace_expr(expr, |expr| {
            if let Expr::Macro(mut expr) = expr {
                let mut e: ExprCall = if expr.mac.path.is_ident("async_stream_block") {
                    syn::parse(async_stream_block(expr.mac.tokens.into())).unwrap()
                } else if expr.mac.path.is_ident("async_try_stream_block") {
                    syn::parse(async_try_stream_block(expr.mac.tokens.into())).unwrap()
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

    // TODO
    // /// Visits `#[stream] async (move) <block>`.
    // fn visit_async(&self, expr: &mut Expr) {
    //     if !self.scope.is_stream() {
    //         return;
    //     }
    //
    //     *expr = if let Expr::Async(expr) = expr {
    //         let mut e: ExprCall = match self.scope {
    //             Stream => syn::parse2(stream::expand_stream_block2(expr)).unwrap(),
    //             TryStream => syn::parse2(try_stream::expand_try_stream_block2(expr)).unwrap(),
    //             _ => unreachable!(),
    //         };
    //         e.attrs.append(&mut expr.attrs);
    //         Expr::Call(e)
    //     } else {
    //         unreachable!()
    //     };
    // }

    /// Visits `<base>.await` in `stream` scope.
    ///
    /// It needs to adjust the type yielded by the macro because generators used internally by
    /// async fn yield `()` type, but generators used internally by `async_stream` yield
    /// `Poll<U>` type.
    fn visit_await(&self, expr: &mut Expr) {
        if !self.scope.is_stream() {
            return;
        }

        // Desugar `<base>.await` into:
        //
        // {
        //     let mut __pinned = <base>;
        //     loop {
        //         if let Poll::Ready(result) = unsafe { Future::poll(
        //             Pin::new_unchecked(&mut __pinned),
        //             get_context(__task_context),
        //         ) } {
        //             break result;
        //         }
        //         __task_context = yield Poll::Pending;
        //     }
        // }
        if let Expr::Await(ExprAwait { base, await_token, .. }) = expr {
            *expr = syn::parse2(quote_spanned! { await_token.span() => {
                let mut __pinned = #base;
                loop {
                    if let ::futures_async_stream::__reexport::task::Poll::Ready(result) =
                    unsafe { ::futures_async_stream::future::Future::poll(
                        ::futures_async_stream::__reexport::pin::Pin::new_unchecked(&mut __pinned),
                        ::futures_async_stream::future::get_context(__task_context),
                    ) } {
                        break result;
                    }
                    __task_context = yield ::futures_async_stream::__reexport::task::Poll::Pending;
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
                    attr.path.is_ident("async_stream") || attr.path.is_ident("async_try_stream")
                }) =>
            {
                self.scope = Skip
            }
            Expr::Async(_) => self.scope = Future,
            Expr::Closure(expr) => {
                self.scope = if expr.asyncness.is_some() { Future } else { Closure }
            }
            Expr::Macro(expr) if expr.mac.path.is_ident("async_stream_block") => {
                self.scope = Stream
            }
            Expr::Macro(expr) if expr.mac.path.is_ident("async_try_stream_block") => {
                self.scope = TryStream
            }
            _ => {}
        }

        if self.scope != Skip {
            visit_mut::visit_expr_mut(self, expr);
            match expr {
                // Expr::Async(_) => self.visit_async(expr),
                Expr::Await(_) => self.visit_await(expr),
                Expr::ForLoop(_) => self.visit_for_loop(expr),
                Expr::Macro(_) => self.visit_macro(expr),
                Expr::Yield(_) => self.visit_yield(expr),
                _ => {}
            }
        }

        // Restore the backup.
        self.scope = tmp;
    }

    fn visit_item_mut(&mut self, _: &mut Item) {
        // Do not recurse into nested items.
    }
}
