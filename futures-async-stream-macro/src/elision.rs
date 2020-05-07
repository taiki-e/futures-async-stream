use proc_macro2::Span;
use syn::{
    punctuated::Punctuated,
    token,
    visit_mut::{self, VisitMut},
    FnArg, GenericArgument, GenericParam, Lifetime, LifetimeDef, Receiver, TypeReference,
};

pub(crate) fn unelide_lifetimes(
    generics: &mut Punctuated<GenericParam, token::Comma>,
    args: &mut Vec<FnArg>,
) {
    let mut visitor = UnelideLifetimes::new(generics);
    args.iter_mut().for_each(|arg| visitor.visit_fn_arg_mut(arg));
}

struct UnelideLifetimes<'a> {
    generics: &'a mut Punctuated<GenericParam, token::Comma>,
    lifetime_index: usize,
    lifetime_name: String,
    count: u32,
}

impl<'a> UnelideLifetimes<'a> {
    fn new(generics: &'a mut Punctuated<GenericParam, token::Comma>) -> Self {
        let lifetime_index = lifetime_index(generics);
        let lifetime_name = lifetime_name(generics);
        Self { generics, lifetime_index, lifetime_name, count: 0 }
    }

    fn next_lifetime(&mut self) -> Lifetime {
        let lifetime_name = format!("{}{}", self.lifetime_name, self.count);
        let lifetime = Lifetime::new(&lifetime_name, Span::call_site());

        let idx = self.lifetime_index + self.count as usize;
        self.generics.insert(idx, GenericParam::Lifetime(LifetimeDef::new(lifetime.clone())));
        self.count += 1;

        lifetime
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

fn lifetime_index(generics: &Punctuated<GenericParam, token::Comma>) -> usize {
    generics
        .iter()
        .take_while(|param| if let GenericParam::Lifetime(_) = param { true } else { false })
        .count()
}

// Determine the prefix for all lifetime names. Ensure it doesn't
// overlap with any existing lifetime names.
fn lifetime_name(generics: &Punctuated<GenericParam, token::Comma>) -> String {
    let mut lifetime_name = String::from("'_async");
    let existing_lifetimes: Vec<String> = generics
        .iter()
        .filter_map(|param| {
            if let GenericParam::Lifetime(LifetimeDef { lifetime, .. }) = param {
                Some(lifetime.to_string())
            } else {
                None
            }
        })
        .collect();
    while existing_lifetimes.iter().any(|name| name.starts_with(&lifetime_name)) {
        lifetime_name.push('_');
    }
    lifetime_name
}
