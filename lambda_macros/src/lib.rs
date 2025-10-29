use std::{fs, io::{self, Write}, path::Path};

use proc_macro::TokenStream;
use quote::ToTokens;
use serde_json::json;
use syn::{parse_macro_input, spanned::Spanned, Error, FnArg, ItemFn, ReturnType};
use toml_edit::{value, DocumentMut, Item};
mod lambda;


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

fn create_binary(crate_root: &str, func: ItemFn) -> Result<(), io::Error> {
    let crate_root_path = Path::new(&crate_root);
    let crate_bin_path = crate_root_path.join("bin");
    if !crate_bin_path.exists() {
        fs::create_dir(&crate_bin_path)?;
    }
    let func_name = func.sig.ident.to_string();
    let lambda_bin_path = crate_bin_path.join(format!("{func_name}.rs"));
    let mut lambda_main_file = fs::File::create(lambda_bin_path)?;

    write_handler(&mut lambda_main_file, crate_root, &func_name, func)?;
    writeln!(lambda_main_file, "#[tokio::main]")?;
    writeln!(lambda_main_file, "async fn main() -> Result<(), Error> {{")?;
    writeln!(lambda_main_file, "  run(service_fn({}_handler)).await", func_name.as_str())?;
    writeln!(lambda_main_file, "}}")?;
    Ok(())
}

fn generate_terraform(crate_root: &str, func_name: &str, lambda: &lambda::Lambda) -> Result<(), io::Error> {
    let terraform_path = Path::new(crate_root).join("terraform");
    
    if !terraform_path.exists() {
        fs::create_dir(&terraform_path)?;
    }
    
    // write api gw deployment template to main.tf.json
    let main_tf_path = terraform_path.join("main.tf.json");
    let mut main_tf_file = fs::File::create(main_tf_path)?;
    writeln!(main_tf_file, "{}", MAIN_TF_TEMPLATE)?;


    let tfvars_path = terraform_path.join("terraform.tfvars.json");
    
    if !tfvars_path.exists() {        
        let mut tfvars_file = fs::File::create(&tfvars_path)?;
        writeln!(tfvars_file, "{}", TERRAFORM_TFVARS_TEMPLATE)?;
    } 
    
    // read existing tfvars file, append to it, and write back to file
    let tfvars_content = fs::read_to_string(&tfvars_path)?;
    let mut tf_vars_json: serde_json::Value = serde_json::from_str(&tfvars_content)?;
    if let Some(lambda_functions) = tf_vars_json.get_mut("lambda_functions") {
        if let Some(obj) = lambda_functions.as_object_mut() {
            obj.insert(func_name.to_string(), json!({
                "name": format!("{func_name}"),
                "code_path": format!("../code_zipped/{func_name}.zip"),
                "s3_key": format!("{func_name}"),
                "api_path": format!("{}", lambda.path),
                "http_method": format!("{}", lambda.http_action),
                "policy_document": format!("../policies/{func_name}.json")
            }));
        }
    }
    fs::write(tfvars_path, serde_json::to_string_pretty(&tf_vars_json)?)?;
    
    Ok(())
}

fn create_bin_and_add_dependencies(crate_root: String, lambda: lambda::Lambda, func: ItemFn) -> Result<(), io::Error> {
    let func_name = func.sig.ident.to_string();
    add_dependencies(&crate_root);
    create_binary(&crate_root, func);
    generate_terraform(&crate_root, &func_name, &lambda)?;
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
    let lambda = parse_macro_input!(attr as lambda::Lambda);
    if let Err(err) = create_bin_and_add_dependencies(crate_root, lambda, func.clone()) {
        return Error::new(func.span(), err.to_string()).to_compile_error().into()
    }
    return func.to_token_stream().into();
}


const TERRAFORM_TFVARS_TEMPLATE: &str = r#"{
  "account_id": "000000000000",
  "api_name": "my-new-api-terraform",
  "s3_bucket_name": "my-code-bucket-terraform-new",
  "lambda_functions": {}
}"#;


const MAIN_TF_TEMPLATE: &str = r#"{
  "terraform": {
    "required_providers": {
      "aws": {
        "source": "hashicorp/aws",
        "version": "~> 5.0"
      }
    }
  },
  "provider": {
    "aws": {
      "access_key": "test",
      "secret_key": "test",
      "region": "us-east-1",
      "skip_credentials_validation": true,
      "skip_metadata_api_check": true,
      "skip_requesting_account_id": true,
      "endpoints": {
        "apigateway": "http://localhost:4566",
        "iam": "http://localhost:4566",
        "lambda": "http://localhost:4566",
        "s3": "http://localhost:4566"
      },
      "s3_use_path_style": true
    }
  },
  "variable": {
    "account_id": {
      "description": "AWS Account ID",
      "type": "string",
      "default": "000000000000"
    },
    "api_name": {
      "description": "API Gateway name",
      "type": "string"
    },
    "s3_bucket_name": {
      "description": "S3 bucket name for Lambda code",
      "type": "string"
    },
    "lambda_functions": {
      "description": "List of Lambda functions to deploy",
      "type": "map(object({name=string,code_path=string,s3_key=string,api_path=string,http_method=string,policy_document=string}))"
    }
  },
  "resource": {
    "aws_s3_bucket": {
      "lambda_code_bucket": {
        "bucket": "${var.s3_bucket_name}"
      }
    },
    "aws_api_gateway_rest_api": {
      "main": {
        "name": "${var.api_name}"
      }
    },
    "aws_s3_object": {
      "lambda_code": {
        "for_each": "${{ for func in var.lambda_functions : func.name => func }}",
        "bucket": "${aws_s3_bucket.lambda_code_bucket.id}",
        "key": "${each.value.s3_key}",
        "source": "${each.value.code_path}",
        "etag": "${filemd5(each.value.code_path)}"
      }
    },
    "aws_iam_role": {
      "function_roles": {
        "for_each": "${{ for func in var.lambda_functions : func.name => func }}",
        "name": "${each.value.name}-role",
        "assume_role_policy": "{\"Version\": \"2012-10-17\", \"Statement\": [{\"Effect\": \"Allow\", \"Principal\": {\"Service\": \"lambda.amazonaws.com\"}, \"Action\": \"sts:AssumeRole\"}]}"
      }
    },
    "aws_iam_role_policy_attachment": {
      "function_basic_execution": {
        "for_each": "${{ for func in var.lambda_functions : func.name => func }}",
        "role": "${aws_iam_role.function_roles[each.key].name}",
        "policy_arn": "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole"
      }
    },
    "aws_iam_role_policy": {
      "function_policies": {
        "for_each": "${{ for func in var.lambda_functions : func.name => func }}",
        "name": "${each.value.name}-policy",
        "role": "${aws_iam_role.function_roles[each.key].id}",
        "policy": "${fileexists(each.value.policy_document) ? file(each.value.policy_document) : each.value.policy_document}"
      }
    },
    "aws_lambda_function": {
      "functions": {
        "for_each": "${{ for func in var.lambda_functions : func.name => func }}",
        "function_name": "${each.value.name}",
        "role": "${aws_iam_role.function_roles[each.key].arn}",
        "handler": "bootstrap",
        "runtime": "provided.al2023",
        "timeout": 30,
        "s3_bucket": "${var.s3_bucket_name}",
        "s3_key": "${each.value.s3_key}",
        "environment": {
          "variables": {
            "AWS_LAMBDA_LOG_LEVEL": "DEBUG"
          }
        },
        "depends_on": [
          "aws_iam_role_policy_attachment.function_basic_execution",
          "aws_iam_role_policy.function_policies",
          "aws_s3_object.lambda_code"
        ]
      }
    },
    "aws_api_gateway_resource": {
      "function_resources": {
        "for_each": "${{ for func in var.lambda_functions : func.name => func }}",
        "rest_api_id": "${aws_api_gateway_rest_api.main.id}",
        "parent_id": "${aws_api_gateway_rest_api.main.root_resource_id}",
        "path_part": "${each.value.api_path}"
      }
    },
    "aws_api_gateway_method": {
      "function_methods": {
        "for_each": "${{ for func in var.lambda_functions : func.name => func }}",
        "rest_api_id": "${aws_api_gateway_rest_api.main.id}",
        "resource_id": "${aws_api_gateway_resource.function_resources[each.key].id}",
        "http_method": "${each.value.http_method}",
        "authorization": "NONE"
      }
    },
    "aws_api_gateway_integration": {
      "function_integrations": {
        "for_each": "${{ for func in var.lambda_functions : func.name => func }}",
        "rest_api_id": "${aws_api_gateway_rest_api.main.id}",
        "resource_id": "${aws_api_gateway_resource.function_resources[each.key].id}",
        "http_method": "${aws_api_gateway_method.function_methods[each.key].http_method}",
        "integration_http_method": "POST",
        "type": "AWS_PROXY",
        "uri": "${aws_lambda_function.functions[each.key].invoke_arn}"
      }
    },
    "aws_lambda_permission": {
      "function_permissions": {
        "for_each": "${{ for func in var.lambda_functions : func.name => func }}",
        "statement_id": "AllowExecutionFromAPIGateway-${each.key}",
        "action": "lambda:InvokeFunction",
        "function_name": "${aws_lambda_function.functions[each.key].function_name}",
        "principal": "apigateway.amazonaws.com",
        "source_arn": "${aws_api_gateway_rest_api.main.execution_arn}/*/*"
      }
    },
    "aws_api_gateway_deployment": {
      "main": {
        "depends_on": [
          "aws_api_gateway_integration.function_integrations"
        ],
        "rest_api_id": "${aws_api_gateway_rest_api.main.id}",
        "stage_name": "$default"
      }
    }
  },
  "output": {
    "function_urls": {
      "description": "URLs for all Lambda functions",
      "value": "${{ for func in var.lambda_functions : func.name => \"http://localhost:4566/restapis/${aws_api_gateway_rest_api.main.id}/$default/_user_request_/${func.api_path}\" }}"    
    },
    "function_arns": {
      "description": "ARNs of all Lambda functions",
      "value": "{ for func in var.lambda_functions : func.name => aws_lambda_function.functions[func.name].arn }"
    },
    "api_gateway_id": {
      "description": "ID of the API Gateway",
      "value": "${aws_api_gateway_rest_api.main.id}"
    }
  }
}"#;

