use serde::{Deserialize, Serialize};
use std::str::FromStr;

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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Policy {
    pub effect: Effect,
    pub action: Action,
    pub resource: String,
}

impl Policy {
    pub fn new(effect: Effect, action: Action, resource: String) -> Self {
        Self {
            effect,
            action,
            resource
        }
    }
}

