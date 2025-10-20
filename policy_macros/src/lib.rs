use proc_macro::TokenStream;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{Write};
use std::path::Path;
use syn::{parse_macro_input, ItemFn};

mod parser;
mod policy;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PolicyForFn {
    policy: policy::Policy,
    fn_name: String,
    // maybe some hash of fn body?
}

#[proc_macro_attribute]
pub fn policy_attr(attr: TokenStream, item: TokenStream) -> TokenStream {
    // parse attr to PolicyForFn struct
    let item_clone = item.clone();  // we need to return this unchanged at end, so cloning
    let func = parse_macro_input!(item_clone as ItemFn);
    let func_name = func.sig.ident.to_string();

    let policy = parse_macro_input!(attr as policy::Policy);
    let policy_for_fn = PolicyForFn {
        policy: policy,
        fn_name: func_name.clone(),
    };

    // get existing policies
    let policy_file = Path::new("./output/policies.json");
    if let Some(parent) = policy_file.parent() {
       fs::create_dir_all(parent).expect("Failed to create output directory");
    }
    let mut policies: Vec<PolicyForFn> = vec![];
    if policy_file.exists() {
        let content = fs::read_to_string(&policy_file).unwrap_or_else(|_| "[]".to_string());
        policies = serde_json::from_str(&content).unwrap_or_else(|_| vec![])
    };

    // update existing policy or add new one
    if let Some(existing) = policies.iter_mut().find(|p| p.fn_name == policy_for_fn.fn_name) {
        *existing = policy_for_fn.clone();
    } else {
        policies.push(policy_for_fn.clone());
    }

    // write back to file
    let json_content =
        serde_json::to_string_pretty(&policies).expect("Failed to serialize policies");
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(&policy_file)
        .expect("Failed to open file");
    file.write_all(json_content.as_bytes()).expect("Failed to write to file");

    // Return function unchanged
    return item;
}


