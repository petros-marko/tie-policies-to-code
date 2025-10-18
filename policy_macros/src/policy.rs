use serde::{Deserialize, Serialize};
use syn::{parse::{Parse, ParseStream}, Ident};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Policy {
    effect: String,
    action: String,
    resource: String,
}

impl Parse for Policy {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let effect = input.parse::<Ident>()?;
        let action = input.parse::<Ident>()?;
        let resource = input.parse::<Ident>()?;
        Ok(Policy {
            effect: effect.to_string(),
            action: action.to_string(),
            resource: resource.to_string(),
        })
    }
}

