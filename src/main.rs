mod s3;

use std::io;

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    let client = s3::get_s3_client().await;
    let mut user_input = String::new();
    loop {
        user_input.clear();
        io::stdin().read_line(&mut user_input).expect("failed to read input");
        let cmd: Vec<&str> = user_input.split_whitespace().collect();

        if cmd.is_empty() {
            println!(
            "Commands: 
                create_bucket <bucket name>
                upload_object <bucket name> <file name> <key>
                download_object <bucket name> <key>
                list_objects <bucket name>
                quit");
            continue;
        }

        match cmd[0].trim() {
            "create_bucket" => {
                if cmd.len() == 2 {
                    let bucket_name = cmd[1]; 
                    s3::create_bucket(&client, bucket_name).await.unwrap();
                    println!("created bucket with name {}", bucket_name);
                } else {
                    println!("create_bucket requires 1 argument");
                }  
            },
            "upload_object" => {
                if cmd.len() == 4 {
                    let bucket_name = cmd[1]; 
                    let file_name = cmd[2];
                    let key = cmd[3];
                    s3::upload_object(&client, bucket_name, file_name, key).await.unwrap();
                    println!("uploaded {} to {}", bucket_name, file_name);
                } else {
                    println!("upload_object requires 3 arguments");
                }
            },
            "download_object" => {
                if cmd.len() == 3 {
                    let bucket_name = cmd[1]; 
                    let key = cmd[2];
                    s3::download_object(&client, bucket_name, key).await.unwrap();
                    println!("downloaded from {}", bucket_name);
                } else {
                    println!("download_object requires 2 arguments");
                }
            },
            "quit" => { break; },
            _ => println!(
                "Commands: 
                create_bucket <bucket name>
                upload_object <bucket name> <file name> <key>
                download_object <bucket name> <key>
                list_objects <bucket name>
                quit"),
        }
    }

    Ok(())
}