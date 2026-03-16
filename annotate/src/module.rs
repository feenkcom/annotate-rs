use crate::function::Function;
use crate::internal::environment::ProtoEnvironment;
use crate::internal::module::ProtoModule;
use crate::{Attribute, Path};
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy)]
pub struct Module {
    pub(crate) environment: &'static ProtoEnvironment,
    pub(crate) proto_module: &'static ProtoModule,
}

impl Module {
    pub const fn name(&self) -> &'static str {
        self.proto_module.name
    }

    pub const fn path(&self) -> &Path {
        &self.proto_module.path
    }

    pub fn find_functions_such_that(
        &self,
        f: impl Fn(&Function) -> bool,
    ) -> Vec<Function> {
        self.functions()
            .filter(|function| f(function))
            .collect()
    }

    pub fn find_modules_such_that(
        &self,
        f: impl Fn(&Module) -> bool,
    ) -> Vec<Module> {
        self.modules()
            .filter(|function| f(function))
            .collect()
    }

    pub fn find_attributes_such_that(
        &self,
        f: impl Fn(&Attribute) -> bool,
    ) -> Vec<&'static Attribute> {
        self.attributes().into_iter().filter(|attribute| f(attribute)).collect()
    }

    pub fn has_attribute_such_that(&self, f: impl Fn(&Attribute) -> bool) -> bool {
        self.attributes().into_iter().any(f)
    }
}
