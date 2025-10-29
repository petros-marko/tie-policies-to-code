use lambda_macros;
use policy_macros;


#[policy_macros::policy_attr(allow get mybucket)]
#[lambda_macros::lambda(GET "mypath")]
pub async fn my_test(
    myarg: String
) -> String {
    return myarg.to_string()
}
