// SPDX-License-Identifier: Apache-2.0 OR MIT

use quote::quote;
use syn::{
    parse_quote, parse_quote_spanned,
    spanned::Spanned as _,
    visit_mut::{self, VisitMut},
    Expr, ExprAwait, ExprCall, ExprForLoop, ExprYield, Item, Token,
};

use crate::{
    parse, stream, stream_block, try_stream_block,
    utils::{expr_compile_error, replace_expr, unit, SliceExt as _},
};

/// The scope in which `#[for_await]`, `.await`, or `yield` was called.
///
/// The type of coroutine depends on which scope is called.
#[derive(Clone, Copy, PartialEq)]
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
        matches!(self, Self::Stream | Self::TryStream)
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

            // It needs to adjust the type yielded by the macro because coroutines used internally by
            // async fn yield `()` type, but coroutines used internally by `stream` yield
            // `Poll<U>` type.
            let match_next = match self.scope {
                Scope::Future => {
                    quote! {
                        match ::futures_async_stream::__private::stream::next(&mut #pinned).await {
                            ::futures_async_stream::__private::Some(e) => e,
                            ::futures_async_stream::__private::None => break,
                        }
                    }
                }
                Scope::Stream | Scope::TryStream => {
                    let task_context = def_site_ident!("__task_context");
                    let poll_result = def_site_ident!("__poll_result");
                    quote! {{
                        let #poll_result = unsafe {
                            ::futures_async_stream::__private::stream::Stream::poll_next(
                                ::futures_async_stream::__private::Pin::as_mut(&mut #pinned),
                                ::futures_async_stream::__private::future::get_context(
                                    #task_context,
                                ),
                            )
                        };
                        match #poll_result {
                            ::futures_async_stream::__private::Poll::Ready(
                                ::futures_async_stream::__private::Some(e),
                            ) => e,
                            ::futures_async_stream::__private::Poll::Ready(
                                ::futures_async_stream::__private::None,
                            ) => break,
                            ::futures_async_stream::__private::Poll::Pending => {
                                #task_context =
                                    yield ::futures_async_stream::__private::Poll::Pending;
                                continue;
                            }
                        }
                    }}
                }
                Scope::Closure => {
                    *expr = expr_compile_error(&format_err!(
                        &expr,
                        "for await may not be allowed outside of \
                         async blocks, functions, closures, async stream blocks, and functions",
                    ));
                    return;
                }
                Scope::Other => unreachable!(),
            };

            body.stmts.insert(0, parse_quote!(let #pat = #match_next;));
            *expr = parse_quote! {{
                let mut #pinned = #e;
                let mut #pinned = unsafe {
                    ::futures_async_stream::__private::Pin::new_unchecked(&mut #pinned)
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

        // Desugar `yield <e>` into `__task_context = yield Poll::Ready(<e>)`.
        if let Expr::Yield(ExprYield { yield_token, expr: e, .. }) = expr {
            e.get_or_insert_with(|| Box::new(unit()));

            let task_context = def_site_ident!("__task_context");
            *expr = parse_quote! {
                #task_context = #yield_token ::futures_async_stream::__private::Poll::Ready(#e)
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
                let mut e: ExprCall =
                    if path_eq(&expr.mac.path, &["futures_async_stream"], &["stream_block"]) {
                        syn::parse(stream_block(expr.mac.tokens.into())).unwrap()
                    } else if path_eq(&expr.mac.path, &["futures_async_stream"], &[
                        "try_stream_block",
                    ]) {
                        syn::parse(try_stream_block(expr.mac.tokens.into())).unwrap()
                    } else {
                        return Expr::Macro(expr);
                    };
                e.attrs.append(&mut expr.attrs);
                Expr::Call(e)
            } else {
                unreachable!()
            }
        });
    }

    /// Visits `#[stream] async (move) <block>`.
    fn visit_async(&self, expr: &mut Expr) {
        if self.scope != Scope::Other {
            return;
        }

        if let Expr::Async(e) = expr {
            // TODO: accept futures_async_stream::{stream,try_stream}
            match (e.attrs.position_exact("stream"), e.attrs.position_exact("try_stream")) {
                (Err(e), _) | (_, Err(e)) => {
                    *expr = expr_compile_error(&e);
                }
                (Ok(Some(_)), Ok(Some(i))) => {
                    *expr = expr_compile_error(&format_err!(
                        e.attrs.remove(i),
                        "#[stream] and #[try_stream] may not be used at the same time"
                    ));
                }
                (Ok(Some(i)), _) => {
                    e.attrs.remove(i);
                    *expr = syn::parse2(stream::parse_async(e, parse::Context::Stream)).unwrap();
                }
                (_, Ok(Some(i))) => {
                    e.attrs.remove(i);
                    *expr = syn::parse2(stream::parse_async(e, parse::Context::TryStream)).unwrap();
                }
                (Ok(None), Ok(None)) => unreachable!(),
            }
        }
    }

    /// Visits `<base>.await`.
    ///
    /// It needs to adjust the type yielded by the macro because coroutines used internally by
    /// async fn yield `()` type, but coroutines used internally by `stream` yield
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
        //             Future::poll(Pin::as_mut(&mut __pinned), get_context(__task_context))
        //         } {
        //             break result;
        //         }
        //         __task_context = yield Poll::Pending;
        //     }
        // }
        if let Expr::Await(ExprAwait { base, await_token, .. }) = expr {
            let task_context = def_site_ident!("__task_context");
            // For interoperability with `forbid(unsafe_code)`, `unsafe` token should be call-site span.
            let unsafety = <Token![unsafe]>::default();
            *expr = parse_quote_spanned! { await_token.span() => {
                let mut __pinned = #base;
                let mut __pinned = #unsafety {
                    ::futures_async_stream::__private::Pin::new_unchecked(&mut __pinned)
                };
                loop {
                    if let ::futures_async_stream::__private::Poll::Ready(result) = #unsafety {
                        ::futures_async_stream::__private::future::Future::poll(
                            ::futures_async_stream::__private::Pin::as_mut(&mut __pinned),
                            ::futures_async_stream::__private::future::get_context(#task_context),
                        )
                    } {
                        break result;
                    }
                    #task_context = yield ::futures_async_stream::__private::Poll::Pending;
                }
            }};
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
                    path_eq(attr.path(), &["futures_async_stream"], &["stream"])
                        || path_eq(attr.path(), &["futures_async_stream"], &["try_stream"])
                }) =>
            {
                self.scope = Scope::Other;
            }
            Expr::Async(_) => {
                self.scope = Scope::Future;
            }
            Expr::Closure(expr) => {
                self.scope = if expr.asyncness.is_some() { Scope::Future } else { Scope::Closure };
            }
            Expr::Macro(expr)
                if path_eq(&expr.mac.path, &["futures_async_stream"], &["stream_block"])
                    || path_eq(&expr.mac.path, &["futures_async_stream"], &[
                        "try_stream_block",
                    ]) =>
            {
                self.scope = Scope::Other;
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

fn path_eq(path: &syn::Path, expected_crates: &[&str], expected_path: &[&str]) -> bool {
    if path.segments.len() == 1 && path.segments[0].ident == expected_path.last().unwrap() {
        return true;
    }
    if path.segments.len() == expected_path.len() + 1 {
        if !expected_crates.iter().any(|&c| path.segments[0].ident == c) {
            return false;
        }
        for i in 1..path.segments.len() {
            if path.segments[i].ident != expected_path[i - 1] {
                return false;
            }
        }
        return true;
    }
    false
}
