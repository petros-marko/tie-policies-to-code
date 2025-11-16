use crate::compiler::PolicyCompiler;
use crate::policy::Policy;

pub struct IamPolicyCompiler {}

impl PolicyCompiler for IamPolicyCompiler {
    fn compile_policy(&self, policy: &Policy) -> String {
        String::from("")
    }
}