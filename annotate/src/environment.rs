use alloc::vec::Vec;
use crate::{Function, Module};

#[derive(Clone, Debug)]
pub struct Environment {
    pub(crate) modules: &'static [Module],
    pub(crate) functions: &'static [Function],
}

impl Environment {

    pub fn find_functions_such_that(&self, f: impl Fn(&Function) -> bool) -> Vec<Function> {
        self.functions.iter().filter(|function| f(function)).cloned().collect()
    }

    pub fn find_modules_such_that(&self, f: impl Fn(&Module) -> bool) -> Vec<Module> {
        self.modules.iter().filter(|module| f(module)).copied().collect()
    }
}
