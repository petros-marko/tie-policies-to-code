use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use syn::{parse::{Parse, ParseStream}, Ident, LitStr, Token};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Lambda {
    pub http_action: HttpAction,
    pub path: String,
}

impl Parse for Lambda {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let http_action = input.parse::<HttpAction>()?;
        // Try to parse comma, but don't require it
        let _ = input.parse::<Token![,]>();
        let path_lit = input.parse::<LitStr>()?;
        let path = path_lit.value();
        Ok(Lambda { http_action, path })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum HttpAction {
    GET,
    POST,
    PUT,
    DELETE
}

impl fmt::Display for HttpAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HttpAction::GET => write!(f, "GET"),
            HttpAction::POST => write!(f, "POST"),
            HttpAction::PUT => write!(f, "PUT"),
            HttpAction::DELETE => write!(f, "DELETE"),
        }
    }
}

impl FromStr for HttpAction {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "GET" => Ok(HttpAction::GET),
            "POST" => Ok(HttpAction::POST),
            "PUT" => Ok(HttpAction::PUT),
            "DELETE" => Ok(HttpAction::DELETE),
            s => Err(format!("Invalid HTTP action: {}", s)),
        }
    }
}

impl Parse for HttpAction {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        let action_str = ident.to_string();
        action_str.parse().map_err(|e| syn::Error::new(ident.span(), e))
    }
}

