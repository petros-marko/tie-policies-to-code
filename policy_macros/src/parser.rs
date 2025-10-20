use std::str::FromStr;
use syn::{parse::{Parse, ParseStream}, Ident};
use crate::policy::{Policy, Effect, Action};

impl Parse for Effect {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        Effect::from_str(&ident.to_string())
            .map_err(|err| syn::Error::new(ident.span(), err))
    }
}

impl Parse for Action {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        Action::from_str(&ident.to_string())
            .map_err(|err| syn::Error::new(ident.span(), err))
    }
}

impl Parse for Policy {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let effect = input.parse::<Effect>()?;
        let action = input.parse::<Action>()?;
        let resource = input.parse::<Ident>()?;
        Ok(Policy::new(
            effect,
            action,
            resource.to_string()
        ))
    }
}