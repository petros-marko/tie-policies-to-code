use serde::{Deserialize, Serialize};
use std::str::FromStr;
use syn::{parse::{Parse, ParseStream}, Ident};

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum Effect {
    Allow,
    Deny,
}

impl FromStr for Effect {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "allow" => Ok(Effect::Allow),
            "deny" => Ok(Effect::Deny),
            s => Err(format!("Invalid effect: {}", s)),
        }
    }
}

impl Parse for Effect {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        Effect::from_str(&ident.to_string())
            .map_err(|err| syn::Error::new(ident.span(), err))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum Action {
    Get,
    Post,
    Put,
    Delete
}

impl FromStr for Action {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "get" => Ok(Action::Get),
            "post" => Ok(Action::Post),
            "put" => Ok(Action::Put),
            "delete" => Ok(Action::Delete),
            s => Err(format!("Invalid action: {}", s)),
        }
    }
}

impl Parse for Action {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        Action::from_str(&ident.to_string())
            .map_err(|err| syn::Error::new(ident.span(), err))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Policy {
    effect: Effect,
    action: Action,
    resource: String,
}

impl Parse for Policy {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let effect = input.parse::<Effect>()?;
        let action = input.parse::<Action>()?;
        let resource = input.parse::<Ident>()?;
        Ok(Policy {
            effect,
            action,
            resource: resource.to_string(),
        })
    }
}

