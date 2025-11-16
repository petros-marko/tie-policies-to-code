use crate::policy::Policy;

pub trait PolicyCompiler {
    fn compile_policy(&self, policy: &Policy) -> String;
}