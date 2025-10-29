- Run `cargo build`
- Right now, you have to manually create the code_zipped/{your_func_name}.zip file
- Run `./deploy.sh`

- To test:
`aws s3api create-bucket --bucket mhanlon-test --endpoint-url http://localhost:4566`
`aws s3 cp ./README.md s3://mhanlon-test/testfile --endpoint-url http://localhost:4566`
If you go to the url from the terraform output, it should give you back the README contents. Example:
function_urls = {
  "my_test" = "http://localhost:4566/restapis/0gyvh03hli/$default/_user_request_/mypath"
}
