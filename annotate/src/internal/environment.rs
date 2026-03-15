use crate::{Function, Module};

use super::function::ProtoFunction;
use super::module::ProtoModule;

#[derive(Debug, Clone, Copy)]
pub struct ProtoEnvironment {
    pub(crate) modules: &'static [ProtoModule],
    pub(crate) functions: &'static [ProtoFunction],
}

impl ProtoEnvironment {
    pub(crate) const fn get_function(&'static self, index: usize) -> Function {
        Function {
            environment: self,
            proto_function: &self.functions[index],
        }
    }

    pub(crate) const fn get_module(&'static self, index: usize) -> Module {
        Module {
            environment: self,
            proto_module: &self.modules[index],
        }
    }
}
