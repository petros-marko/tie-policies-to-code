use serde::{Deserialize, Serialize};
use syn::{parse::{Parse, ParseStream}, Ident};

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum Effect {
    Allow,
    Deny,
}

impl Parse for Effect {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        match ident.to_string().as_str() {
            "allow" => Ok(Effect::Allow),
            "deny"  => Ok(Effect::Deny),
            _ => Err(syn::Error::new(ident.span(), format!("Encountered invalid effect: {}", ident)))
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Policy {
    effect: Effect,
    action: String,
    resource: String,
}

impl Parse for Policy {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let effect = input.parse::<Effect>()?;
        let action = input.parse::<Ident>()?;
        let resource = input.parse::<Ident>()?;
        Ok(Policy {
            effect,
            action: action.to_string(),
            resource: resource.to_string(),
        })
    }
}

