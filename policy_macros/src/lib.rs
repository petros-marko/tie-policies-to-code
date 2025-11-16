mod parser;
mod policy;
mod compiler;
mod iam_policy_compiler;

use proc_macro::TokenStream;
use serde_json::json;
use syn::spanned::Spanned;
use std::fs::{self, OpenOptions};
use std::io::{Write};
use std::path::Path;
use syn::{parse_macro_input, ItemFn, Error};

use compiler::PolicyCompiler;
use iam_policy_compiler::IamPolicyCompiler;

#[proc_macro_attribute]
pub fn policy_attr(attr: TokenStream, item: TokenStream) -> TokenStream {
    // parse attr to PolicyForFn struct
    let item_clone = item.clone();  // we need to return this unchanged at end, so cloning
    let func = parse_macro_input!(item_clone as ItemFn);
    let func_name = func.sig.ident.to_string();

    let policy = parse_macro_input!(attr as policy::Policy);

    // get existing policies
    let crate_root_res = std::env::var("CARGO_MANIFEST_DIR");
    if let Err(_) = crate_root_res {
        return Error::new(func.span(), "Could not locate crate root").into_compile_error().into();
    }
    let crate_root = crate_root_res.unwrap();
    let crate_root_path = Path::new(&crate_root);
    let crate_policies_path = crate_root_path.join("policies");
    if !crate_policies_path.exists() {
        let create_dir_result = fs::create_dir(&crate_policies_path);
        if let Err(_) = create_dir_result {
            return Error::new(func.span(), "Could not create policies directory").into_compile_error().into();
        }
    }
    let policy_file = crate_policies_path.join(format!("{}.json", func_name));
    if let Some(parent) = policy_file.parent() {
       fs::create_dir_all(parent).expect("Failed to create output directory");
    }
    let mut policies: Vec<policy::Policy> = vec![];
    if policy_file.exists() {
        let content = fs::read_to_string(&policy_file).unwrap_or_else(|_| "[]".to_string());
        policies = serde_json::from_str(&content).unwrap_or_else(|_| vec![])
    };

    policies.push(policy.clone());

    let compiler = IamPolicyCompiler {};
    let iam_json = compiler.compile_policy(&policy);

    // write back to file as iam json
    let json_content =
        serde_json::to_string_pretty(&iam_json).expect("Failed to serialize policies");
        
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(&policy_file)
        .expect("Failed to open file");
    file.write_all(json_content.as_bytes()).expect("Failed to write to file");
    // Return function unchanged
    return item;
}


