use alloc::vec::Vec;
use std::any::TypeId;

use crate::{Attribute, Attributes, Path};
use crate::function::Function;
use crate::internal::environment::ProtoEnvironment;
use crate::internal::function::Functions;
use crate::internal::module::{ProtoModule, Submodules};

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

    pub const fn amount_of_functions(&self) -> usize {
        self.proto_module.functions.len()
    }

    pub fn attributes(&self) -> Attributes {
        Attributes::new(unsafe { (self.proto_module.attributes)() })
    }

    pub fn submodules(&self) -> impl Iterator<Item = Module> {
        Submodules::new(self.environment, self.proto_module.submodules)
    }

    pub fn functions(&self) -> impl Iterator<Item = Function> {
        Functions::new(self.environment, self.proto_module.functions)
    }

    pub fn find_functions_such_that(&self, f: &impl Fn(&Function) -> bool) -> Vec<Function> {
        self.functions().filter(|function| f(function)).collect()
    }

    pub fn has_attribute_such_that(&self, f: &impl Fn(&Attribute) -> bool) -> bool {
        self.attributes().into_iter().any(f)
    }

    pub fn has_type_attribute<T: 'static>(&self, name: &str) -> bool {
        self.has_attribute_such_that(&|attribute| {
            attribute.name() == name && attribute.is_type::<T>()
        })
    }

    pub fn has_type_id_attribute(&self, name: &str, value: &TypeId) -> bool {
        self.has_attribute_such_that(&|attribute| {
            attribute.name() == name && attribute.is_type_id(value)
        })
    }

    pub fn has_string_attribute(self, name: &str, value: &str) -> bool {
        self.has_attribute_such_that(&|attribute| {
            attribute.name() == name && attribute.is_str(value)
        })
    }
}
