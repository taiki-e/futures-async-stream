// SPDX-License-Identifier: Apache-2.0 OR MIT

#![allow(clippy::match_on_vec_items, clippy::needless_pass_by_value, clippy::wildcard_imports)]

#[macro_use]
mod file;

use std::{collections::BTreeSet, path::Path};

use anyhow::Result;
use fs_err as fs;
use quote::{format_ident, quote, ToTokens};
use syn::visit_mut::{self, VisitMut};

use crate::file::*;

fn main() -> Result<()> {
    gen_assert_impl()?;
    Ok(())
}

fn gen_assert_impl() -> Result<()> {
    const NOT_SEND: &[&str] = &[];
    const NOT_SYNC: &[&str] = &[];
    const NOT_UNPIN: &[&str] = &[];
    const NOT_UNWIND_SAFE: &[&str] = &[];
    const NOT_REF_UNWIND_SAFE: &[&str] = &[];

    let workspace_root = &workspace_root();
    let out_dir = &workspace_root.join("src/gen");
    fs::create_dir_all(out_dir)?;

    let files: BTreeSet<String> = git_ls_files(&workspace_root.join("src"), &["*.rs"])?
        .into_iter()
        .filter_map(|(file_name, path)| {
            // Assertions are only needed for the library's public APIs.
            if file_name == "main.rs" || file_name.starts_with("bin/") {
                return None;
            }
            Some(path.to_string_lossy().into_owned())
        })
        .collect();

    let mut tokens = quote! {};
    let mut visited_types = BTreeSet::new();
    let mut use_macros = false;
    let mut use_generics_helpers = false;
    for f in &files {
        let s = fs::read_to_string(f)?;
        let mut ast = syn::parse_file(&s)?;

        let module = if f.ends_with("lib.rs") {
            vec![]
        } else {
            let name = format_ident!("{}", Path::new(f).file_stem().unwrap().to_string_lossy());
            vec![name.into()]
        };

        // TODO: assert impl trait returned from public functions
        ItemVisitor::new(module, |item, module| match item {
            syn::Item::Struct(syn::ItemStruct { vis, ident, generics, .. })
            | syn::Item::Enum(syn::ItemEnum { vis, ident, generics, .. })
            | syn::Item::Union(syn::ItemUnion { vis, ident, generics, .. })
            | syn::Item::Type(syn::ItemType { vis, ident, generics, .. })
                if matches!(vis, syn::Visibility::Public(..)) =>
            {
                let path_string = quote! { #(#module::)* #ident }.to_string().replace(' ', "");
                visited_types.insert(path_string.clone());

                let has_generics = generics.type_params().count() != 0;
                assert_eq!(
                    generics.const_params().count(),
                    0,
                    "gen_assert_impl doesn't support const generics yet; skipped `{path_string}`"
                );

                let lt = generics.lifetimes().map(|_| quote! { '_ }).collect::<Vec<_>>();
                if has_generics {
                    use_macros = true;
                    use_generics_helpers = true;
                    // Send & Sync & Unpin
                    let unit = generics.type_params().map(|_| quote! { () }).collect::<Vec<_>>();
                    let unit_generics = quote! { <#(#lt,)* #(#unit),*> };
                    // !Send & !Sync
                    let not_send_sync =
                        generics.type_params().map(|_| quote! { NotSendSync }).collect::<Vec<_>>();
                    let not_send_sync_generics = quote! { <#(#lt,)* #(#not_send_sync),*> };
                    // Send & !Sync
                    let not_sync =
                        generics.type_params().map(|_| quote! { NotSync }).collect::<Vec<_>>();
                    let not_sync_generics = quote! { <#(#lt,)* #(#not_sync),*> };
                    // !Unpin
                    let not_unpin = generics
                        .type_params()
                        .map(|_| quote! { PhantomPinned })
                        .collect::<Vec<_>>();
                    let not_unpin_generics = quote! { <#(#lt,)* #(#not_unpin),*> };
                    // !UnwindSafe
                    let not_unwind_safe = generics
                        .type_params()
                        .map(|_| quote! { NotUnwindSafe })
                        .collect::<Vec<_>>();
                    let not_unwind_safe_generics = quote! { <#(#lt,)* #(#not_unwind_safe),*> };
                    // !RefUnwindSafe
                    let not_ref_unwind_safe = generics
                        .type_params()
                        .map(|_| quote! { NotRefUnwindSafe })
                        .collect::<Vec<_>>();
                    let not_ref_unwind_safe_generics =
                        quote! { <#(#lt,)* #(#not_ref_unwind_safe),*> };
                    if NOT_SEND.contains(&path_string.as_str()) {
                        tokens.extend(quote! {
                            assert_not_send!(crate:: #(#module::)* #ident #unit_generics);
                        });
                    } else {
                        tokens.extend(quote! {
                            assert_send::<crate:: #(#module::)* #ident #unit_generics>();
                            assert_send::<crate:: #(#module::)* #ident #not_sync_generics>();
                            assert_not_send!(crate:: #(#module::)* #ident #not_send_sync_generics);
                        });
                    }
                    if NOT_SYNC.contains(&path_string.as_str()) {
                        tokens.extend(quote! {
                            assert_not_sync!(crate:: #(#module::)* #ident #unit_generics);
                        });
                    } else {
                        tokens.extend(quote! {
                            assert_sync::<crate:: #(#module::)* #ident #unit_generics>();
                            assert_not_sync!(crate:: #(#module::)* #ident #not_sync_generics);
                            assert_not_sync!(crate:: #(#module::)* #ident #not_send_sync_generics);
                        });
                    }
                    if NOT_UNPIN.contains(&path_string.as_str()) {
                        tokens.extend(quote! {
                            assert_not_unpin!(crate:: #(#module::)* #ident #unit_generics);
                        });
                    } else {
                        tokens.extend(quote! {
                            assert_unpin::<crate:: #(#module::)* #ident #unit_generics>();
                            assert_not_unpin!(crate:: #(#module::)* #ident #not_unpin_generics);
                        });
                    }
                    if NOT_UNWIND_SAFE.contains(&path_string.as_str()) {
                        tokens.extend(quote! {
                            assert_not_unwind_safe!(crate:: #(#module::)* #ident #unit_generics);
                        });
                    } else {
                        tokens.extend(quote! {
                            assert_unwind_safe::<crate:: #(#module::)* #ident #unit_generics>();
                            assert_not_unwind_safe!(
                                crate:: #(#module::)* #ident #not_unwind_safe_generics
                            );
                        });
                    }
                    if NOT_REF_UNWIND_SAFE.contains(&path_string.as_str()) {
                        tokens.extend(quote! {
                            assert_not_ref_unwind_safe!(
                                crate:: #(#module::)* #ident #unit_generics
                            );
                        });
                    } else {
                        tokens.extend(quote! {
                            assert_ref_unwind_safe::<crate:: #(#module::)* #ident #unit_generics>();
                            assert_not_ref_unwind_safe!(
                                crate:: #(#module::)* #ident #not_ref_unwind_safe_generics
                            );
                        });
                    }
                } else {
                    let lt = if lt.is_empty() {
                        quote! {}
                    } else {
                        quote! { <#(#lt),*> }
                    };
                    if NOT_SEND.contains(&path_string.as_str()) {
                        use_macros = true;
                        tokens.extend(quote! {
                            assert_not_send!(crate:: #(#module::)* #ident #lt);
                        });
                    } else {
                        tokens.extend(quote! {
                            assert_send::<crate:: #(#module::)* #ident #lt>();
                        });
                    }
                    if NOT_SYNC.contains(&path_string.as_str()) {
                        use_macros = true;
                        tokens.extend(quote! {
                            assert_not_sync!(crate:: #(#module::)* #ident #lt);
                        });
                    } else {
                        tokens.extend(quote! {
                            assert_sync::<crate:: #(#module::)* #ident #lt>();
                        });
                    }
                    if NOT_UNPIN.contains(&path_string.as_str()) {
                        use_macros = true;
                        tokens.extend(quote! {
                            assert_not_unpin!(crate:: #(#module::)* #ident #lt);
                        });
                    } else {
                        tokens.extend(quote! {
                            assert_unpin::<crate:: #(#module::)* #ident #lt>();
                        });
                    }
                    if NOT_UNWIND_SAFE.contains(&path_string.as_str()) {
                        use_macros = true;
                        tokens.extend(quote! {
                            assert_not_unwind_safe!(crate:: #(#module::)* #ident #lt);
                        });
                    } else {
                        tokens.extend(quote! {
                            assert_unwind_safe::<crate:: #(#module::)* #ident #lt>();
                        });
                    }
                    if NOT_REF_UNWIND_SAFE.contains(&path_string.as_str()) {
                        use_macros = true;
                        tokens.extend(quote! {
                            assert_not_ref_unwind_safe!(crate:: #(#module::)* #ident #lt);
                        });
                    } else {
                        tokens.extend(quote! {
                            assert_ref_unwind_safe::<crate:: #(#module::)* #ident #lt>();
                        });
                    }
                };
            }
            _ => {}
        })
        .visit_file_mut(&mut ast);
    }

    for &t in NOT_SEND {
        assert!(visited_types.contains(t), "unknown type `{t}` specified in NOT_SEND constant");
    }
    for &t in NOT_SYNC {
        assert!(visited_types.contains(t), "unknown type `{t}` specified in NOT_SYNC constant");
    }
    for &t in NOT_UNPIN {
        assert!(visited_types.contains(t), "unknown type `{t}` specified in NOT_UNPIN constant");
    }
    for &t in NOT_UNWIND_SAFE {
        assert!(
            visited_types.contains(t),
            "unknown type `{t}` specified in NOT_UNWIND_SAFE constant"
        );
    }
    for &t in NOT_REF_UNWIND_SAFE {
        assert!(
            visited_types.contains(t),
            "unknown type `{t}` specified in NOT_REF_UNWIND_SAFE constant"
        );
    }

    let mut out = quote! {
        #![allow(
            clippy::std_instead_of_alloc,
            clippy::std_instead_of_core,
        )]
        #[allow(dead_code)]
        fn assert_send<T: ?Sized + Send>() {}
        #[allow(dead_code)]
        fn assert_sync<T: ?Sized + Sync>() {}
        #[allow(dead_code)]
        fn assert_unpin<T: ?Sized + Unpin>() {}
        #[allow(dead_code)]
        fn assert_unwind_safe<T: ?Sized + std::panic::UnwindSafe>() {}
        #[allow(dead_code)]
        fn assert_ref_unwind_safe<T: ?Sized + std::panic::RefUnwindSafe>() {}
    };
    if use_generics_helpers {
        out.extend(quote! {
            #[allow(unused_imports)]
            use core::marker::PhantomPinned;
            /// `Send` & `!Sync`
            #[allow(dead_code)]
            struct NotSync(core::cell::UnsafeCell<()>);
            /// `!Send` & `!Sync`
            #[allow(dead_code)]
            struct NotSendSync(*const ());
            /// `!UnwindSafe`
            #[allow(dead_code)]
            struct NotUnwindSafe(&'static mut ());
            /// `!RefUnwindSafe`
            #[allow(dead_code)]
            struct NotRefUnwindSafe(core::cell::UnsafeCell<()>);
        });
    }
    if use_macros {
        out.extend(quote! {
            #[allow(unused_macros)]
            macro_rules! assert_not_send {
                ($ty:ty) => {
                    static_assertions::assert_not_impl_all!($ty: Send);
                };
            }
            #[allow(unused_macros)]
            macro_rules! assert_not_sync {
                ($ty:ty) => {
                    static_assertions::assert_not_impl_all!($ty: Sync);
                };
            }
            #[allow(unused_macros)]
            macro_rules! assert_not_unpin {
                ($ty:ty) => {
                    static_assertions::assert_not_impl_all!($ty: Unpin);
                };
            }
            #[allow(unused_macros)]
            macro_rules! assert_not_unwind_safe {
                ($ty:ty) => {
                    static_assertions::assert_not_impl_all!($ty: std::panic::UnwindSafe);
                };
            }
            #[allow(unused_macros)]
            macro_rules! assert_not_ref_unwind_safe {
                ($ty:ty) => {
                    static_assertions::assert_not_impl_all!($ty: std::panic::RefUnwindSafe);
                };
            }
        });
    }
    out.extend(quote! {
        const _: fn() = || {
            #tokens
        };
    });
    write(function_name!(), out_dir.join("assert_impl.rs"), out)?;

    Ok(())
}

#[must_use]
struct ItemVisitor<F> {
    module: Vec<syn::PathSegment>,
    f: F,
}

impl<F> ItemVisitor<F>
where
    F: FnMut(&mut syn::Item, &[syn::PathSegment]),
{
    fn new(module: Vec<syn::PathSegment>, f: F) -> Self {
        Self { module, f }
    }
}

impl<F> VisitMut for ItemVisitor<F>
where
    F: FnMut(&mut syn::Item, &[syn::PathSegment]),
{
    fn visit_item_mut(&mut self, item: &mut syn::Item) {
        match item {
            syn::Item::Mod(item) => {
                self.module.push(item.ident.clone().into());
                visit_mut::visit_item_mod_mut(self, item);
                self.module.pop();
            }
            syn::Item::Macro(item) => {
                if let Ok(mut file) = syn::parse2::<syn::File>(item.mac.tokens.clone()) {
                    visit_mut::visit_file_mut(self, &mut file);
                    item.mac.tokens = file.into_token_stream();
                }
                visit_mut::visit_item_macro_mut(self, item);
            }
            _ => {
                (self.f)(item, &self.module);
                visit_mut::visit_item_mut(self, item);
            }
        }
    }
}
