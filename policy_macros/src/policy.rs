use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Action {
    Create, Read, Update, Delete
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Resource {
    Table(String)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Key {
    Pk,
    Sk,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Var(pub Vec<String>);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum StringExpr {
    Literal(String),
    Variable(Var),
    Concat(Box<StringExpr>, Box<StringExpr>)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Filter {
    KeyEquals(Key, StringExpr),
    KeyLike(Key, StringExpr)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Field(pub String);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PolicyAtom {
    pub action: Action,
    pub resource: Resource,
    pub filters: Vec<Filter>,
    pub attributes: Option<Vec<Field>>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Policy {
    Atom(PolicyAtom),
    Composite(Vec<PolicyAtom>)
}