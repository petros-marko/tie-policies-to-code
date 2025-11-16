use syn::{parse::{Parse, ParseStream}, Ident, Token, LitStr, parenthesized, bracketed, punctuated::Punctuated};
use crate::policy::*;

fn next_is_string_value(input: ParseStream, expected: &str) -> bool {
    if !input.peek(Ident) {
        return false;
    }
    let fork = input.fork();
    match fork.parse::<Ident>() {
        Ok(lit) => lit.to_string().as_str() == expected,
        Err(_) => false,
    }
}

fn parse_and_ignore(input: ParseStream, word: &str) -> syn::Result<()> {
    if next_is_string_value(input, word) {
        input.parse::<Ident>()?;
        Ok(())
    } else {
        Err(input.error(format!("expected '{word}'").as_str()))
    }
}

impl Parse for Action {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let action_ident: Ident = input.parse()?;
        let action_str = action_ident.to_string();
        match action_str.as_str() {
            "create" => Ok(Action::Create),
            "read" => Ok(Action::Read),
            "update" => Ok(Action::Update),
            "delete" => Ok(Action::Delete),
            _ => Err(input.error(format!("unexpected action: '{action_str}'").as_str()))
        }
    }
}

impl Parse for Resource {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        parse_and_ignore(input, "table")?;
        let table_name: LitStr = input.parse()?;
        Ok(Resource::Table(table_name.value()))
    }
}

impl Parse for Key {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<Token![$]>()?;
        let key_str = input.parse::<Ident>()?.to_string();
        match key_str.as_str() {
            "pk" => Ok(Key::Pk),
            "sk" => Ok(Key::Sk),
            _ => Err(input.error(format!("unknown key encountered '${key_str}'").as_str()))
        }
    }
}

impl Parse for Var {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<Token![$]>()?;
        let idents: Punctuated<Ident, Token![.]> =
            Punctuated::parse_separated_nonempty(input)?;
        let var_path: Vec<String> = idents.into_iter().map(|ident| ident.to_string()).collect();
        Ok(Var(var_path))
    }
}

impl Parse for StringExpr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(LitStr) {
            let lit: LitStr = input.parse()?;
            return Ok(StringExpr::Literal(lit.value()));
        }

        if input.peek(Token![$]) {
            input.parse::<Token![$]>()?;
            let var = input.parse::<Var>()?;
            return Ok(StringExpr::Variable(var));
        }

        if next_is_string_value(input, "concat") {
            input.parse::<Ident>()?;
            let content;
            parenthesized!(content in input);
            let left = content.parse::<StringExpr>()?;
            content.parse::<Token![,]>()?;
            let right = content.parse::<StringExpr>()?;
            return Ok(StringExpr::Concat(Box::new(left), Box::new(right)));
        }

        Err(input.error("invalid string expression"))
    }
}

impl Parse for Filter {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        match input.parse::<Ident>()?.to_string().as_str() {
            "key_equals" => {
                let key = input.parse::<Key>()?;
                let str_expr = input.parse::<StringExpr>()?;
                Ok(Filter::KeyEquals(key, str_expr))
            }
            "key_like" => {
                let key = input.parse::<Key>()?;
                let str_expr = input.parse::<StringExpr>()?;
                Ok(Filter::KeyLike(key, str_expr))
            }
            _ => {
                Err(input.error("expected one of ['key_equals', 'key_like']"))
            }
        }
    }
}

impl Parse for Field {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lit = input.parse::<LitStr>()?;
        Ok(Field(lit.value()))
    }
}

impl Parse for PolicyAtom {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        parse_and_ignore(input, "allow")?;
        let action = input.parse::<Action>()?;
        parse_and_ignore(input, "on")?;
        let resource = input.parse::<Resource>()?;
        let mut filters = Vec::new();
        while input.peek(Token![where]) {
            input.parse::<Token![where]>()?;
            filters.push(input.parse::<Filter>()?);
        }
        let fields = if next_is_string_value(input, "with") {
            input.parse::<Ident>()?;
            parse_and_ignore(input, "attributes")?;
            let content;
            bracketed!(content in input);
            let mut fields = Vec::new();
            while !content.is_empty() {
                fields.push(content.parse::<Field>()?);
            }
            Some(fields)
        } else {
            None
        };
        Ok(PolicyAtom {
            action,
            resource,
            filters,
            attributes: fields
        })
    }
}

impl Parse for Policy {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut policy_atoms = vec![];
        while let Ok(policy_atom) = input.parse::<PolicyAtom>() {
            policy_atoms.push(policy_atom);
        }
        if policy_atoms.len() == 1 {
            Ok(Policy::Atom(policy_atoms.pop().unwrap()))
        } else {
            Ok(Policy::Composite(policy_atoms))
        }
    }
}