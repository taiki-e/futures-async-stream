use proc_macro2::Span;
use syn::{
    punctuated::Punctuated,
    visit_mut::{self, VisitMut},
    FnArg, GenericArgument, GenericParam, Generics, Lifetime, LifetimeDef, Receiver, Token,
    TypeReference,
};

pub(crate) fn unelide_lifetimes(generics: &mut Generics, args: &mut Vec<FnArg>) {
    let mut visitor = UnelideLifetimes::new(generics);
    args.iter_mut().for_each(|arg| visitor.visit_fn_arg_mut(arg));
}

struct UnelideLifetimes<'a> {
    generics: &'a mut Punctuated<GenericParam, Token![,]>,
    lifetime_index: usize,
    lifetime_name: String,
    count: u32,
}

impl<'a> UnelideLifetimes<'a> {
    fn new(generics: &'a mut Generics) -> Self {
        let lifetime_index = generics.lifetimes().count();
        let lifetime_name = determine_lifetime_name(generics);
        Self { generics: &mut generics.params, lifetime_index, lifetime_name, count: 0 }
    }

    fn visit_opt_lifetime(&mut self, lifetime: &mut Option<Lifetime>) {
        match lifetime {
            None => *lifetime = Some(self.next_lifetime()),
            Some(lifetime) => self.visit_lifetime(lifetime),
        }
    }

    fn visit_lifetime(&mut self, lifetime: &mut Lifetime) {
        if lifetime.ident == "_" {
            *lifetime = self.next_lifetime();
        }
    }

    fn next_lifetime(&mut self) -> Lifetime {
        let lifetime_name = format!("{}{}", self.lifetime_name, self.count);
        let lifetime = Lifetime::new(&lifetime_name, Span::call_site());

        let idx = self.lifetime_index + self.count as usize;
        self.generics.insert(idx, LifetimeDef::new(lifetime.clone()).into());
        self.count += 1;

        lifetime
    }
}

impl VisitMut for UnelideLifetimes<'_> {
    fn visit_receiver_mut(&mut self, receiver: &mut Receiver) {
        if let Some((_, lifetime)) = &mut receiver.reference {
            self.visit_opt_lifetime(lifetime);
        }
    }

    fn visit_type_reference_mut(&mut self, ty: &mut TypeReference) {
        self.visit_opt_lifetime(&mut ty.lifetime);
        visit_mut::visit_type_reference_mut(self, ty);
    }

    fn visit_generic_argument_mut(&mut self, gen: &mut GenericArgument) {
        if let GenericArgument::Lifetime(lifetime) = gen {
            self.visit_lifetime(lifetime);
        }
        visit_mut::visit_generic_argument_mut(self, gen);
    }
}

/// Determine the prefix for all lifetime names. Ensure it doesn't overlap with
/// any existing lifetime names.
fn determine_lifetime_name(generics: &mut Generics) -> String {
    struct CollectLifetimes(Vec<String>);

    impl VisitMut for CollectLifetimes {
        fn visit_lifetime_def_mut(&mut self, def: &mut LifetimeDef) {
            self.0.push(def.lifetime.to_string());
        }
    }

    let mut lifetime_name = String::from("'_async");

    let mut lifetimes = CollectLifetimes(Vec::new());
    lifetimes.visit_generics_mut(generics);

    while lifetimes.0.iter().any(|name| name.starts_with(&lifetime_name)) {
        lifetime_name.push('_');
    }
    lifetime_name
}
