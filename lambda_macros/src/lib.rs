use std::{fs, io::{self, Write}, path::Path};

use proc_macro::TokenStream;
use quote::ToTokens;
use syn::{parse_macro_input, spanned::Spanned, Error, FnArg, Ident, ItemFn, ReturnType};
use toml_edit::{value, DocumentMut, Item};

fn add_dependencies(crate_root: &str) -> Result<(), io::Error> {
    let crate_root_path = Path::new(crate_root);
    let cargo_toml_path = crate_root_path.join("Cargo.toml");
    let text = fs::read_to_string(&cargo_toml_path)?;
    let mut doc = text.parse::<DocumentMut>().unwrap();
    let dependencies = doc.as_table_mut().entry("dependencies").or_insert(Item::Value(toml_edit::Value::InlineTable(Default::default()))).as_table_mut().unwrap();
    if !dependencies.contains_key("lambda_runtime") {
        dependencies["lambda_runtime"] = value("0.13.0");
    }
    if !dependencies.contains_key("tokio") {
        dependencies["tokio"] = value("{ version = \"1\", features = [\"macros\"] }");
    }
    fs::write(&cargo_toml_path, doc.to_string())
}

fn return_type(func: &ItemFn) -> String {
    match &func.sig.output {
        ReturnType::Default => "()".to_string(),
        ReturnType::Type(_, ty) => ty.to_token_stream().to_string()
    }
}

fn input_type(func: &ItemFn) -> String {
    // We already checked that the function has an argument
    match func.sig.inputs.first().unwrap() {
        FnArg::Receiver(_) => panic!("Annotated bad function"),
        FnArg::Typed(pat_type) => pat_type.ty.to_token_stream().to_string()
    }
}

fn root_crate_name(crate_root: &str) -> Result<String, io::Error> {
    let crate_root_path = Path::new(crate_root);
    let cargo_toml_path = crate_root_path.join("Cargo.toml");
    let text = fs::read_to_string(&cargo_toml_path)?;
    let doc = text.parse::<DocumentMut>().unwrap();
    Ok(doc["package"]["name"].as_str().unwrap().replace("-", "_"))
}

fn write_handler(file: &mut fs::File, crate_root: &str, func_name: &str, func: ItemFn) -> Result<(), io::Error> {
    writeln!(file, "use lambda_runtime::{{run, service_fn, LambdaEvent, Error}};")?;
    writeln!(file, "use {}::*;", root_crate_name(&crate_root)?)?;
    writeln!(file, "")?;
    writeln!(
        file, 
        "{} fn {}_handler(event: LambdaEvent<{}>) -> Result<{}, Error> {{",
        func.sig.asyncness.to_token_stream().to_string(),
        func_name,
        input_type(&func).as_str(),
        return_type(&func).as_str()
    )?;
    writeln!(
        file, 
        "  Ok({}(event.payload).await)",
        func_name
    )?;
    writeln!(file, "}}")?;
    writeln!(file, "")?;
    Ok(())
}

fn create_binary(crate_root: &str, lambda_name: &str, func: ItemFn) -> Result<(), io::Error> {
    let crate_root_path = Path::new(&crate_root);
    let crate_bin_path = crate_root_path.join("src/bin");
    if !crate_bin_path.exists() {
        fs::create_dir(&crate_bin_path)?;
    }
    let lambda_bin_path = crate_bin_path.join(format!("{lambda_name}.rs"));
    let func_name = func.sig.ident.to_string();
    let mut lambda_main_file = fs::File::create(lambda_bin_path)?;

    write_handler(&mut lambda_main_file, crate_root, &func_name, func)?;
    writeln!(lambda_main_file, "#[tokio::main]")?;
    writeln!(lambda_main_file, "async fn main() -> Result<(), Error> {{")?;
    writeln!(lambda_main_file, "  run(service_fn({}_handler)).await", func_name.as_str())?;
    writeln!(lambda_main_file, "}}")?;
    Ok(())
}

fn create_bin_and_add_dependencies(crate_root: String, lambda_name: String, func: ItemFn) -> Result<(), io::Error> {
    add_dependencies(&crate_root)?;
    create_binary(&crate_root, &lambda_name, func)?;
    Ok(())
}


#[proc_macro_attribute]
pub fn lambda(attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = parse_macro_input!(item as ItemFn);
    if func.sig.inputs.len() != 1 {
        return Error::new(
            func.sig.inputs.span(), 
            "Only function with exactly one argument can be deployed on Lambda. Consider packaging the arguments into a struct that implements Deserialize"
            ).into_compile_error().into();
    }
    if func.sig.asyncness.is_none() {
        return Error::new(
            func.sig.asyncness.span(),
            "Only async functions can be deployed on Lambda. Consider marking this function as async."
        ).into_compile_error().into();
    }
    let crate_root = std::env::var("CARGO_MANIFEST_DIR");
    if let Err(_) = crate_root {
        return Error::new(func.span(), "Could not locate crate root").into_compile_error().into();
    }
    let crate_root = crate_root.unwrap();
    let lambda_name = parse_macro_input!(attr as Ident).to_string();
    if let Err(err) = create_bin_and_add_dependencies(crate_root, lambda_name, func.clone()) {
        return Error::new(func.span(), err.to_string()).to_compile_error().into()
    }
    return func.to_token_stream().into();
}
