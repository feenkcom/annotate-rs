use core::any::{TypeId, type_name};
use core::fmt;
use core::fmt::{Debug, Formatter};
use alloc::vec::Vec;

use crate::{Attribute, Environment, Module, Path};
use crate::internal::environment::ProtoEnvironment;
use crate::internal::function::ProtoFunction;

#[derive(Clone)]
pub struct Function {
    pub(crate) environment: &'static ProtoEnvironment,
    pub(crate) proto_function: &'static ProtoFunction,
}

impl Function {
    pub const fn name(&self) -> &'static str {
        self.proto_function.name
    }

    pub const fn module(&self) -> Option<Module> {
        if let Some(id) = self.proto_function.module {
            return Some(self.environment.get_module(id));
        }
        None
    }

    pub const fn path(&self) -> &'static Path {
        &self.proto_function.path
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

    pub fn same_as<F: 'static>(&self) -> bool {
        unsafe { (self.proto_function.function)() }.raw.is::<F>()
    }

    pub fn cast<F: 'static>(&self) -> Result<&'static F, TypeMismatch> {
        unsafe { (self.proto_function.function)() }
            .raw
            .downcast_ref::<F>()
            .ok_or_else(|| TypeMismatch {
                type_name: type_name::<F>(),
                type_id: TypeId::of::<F>(),
                expected_type_id: (unsafe { (self.proto_function.function)() }).raw.type_id(),
            })
    }

    pub fn try_call<F: 'static, R>(&self, invoke: impl FnOnce(&F) -> R) -> Result<R, TypeMismatch> {
        self.cast::<F>().map(invoke)
    }

    pub fn call<F: 'static, R>(&self, invoke: impl FnOnce(&F) -> R) -> R {
        self.try_call::<F, R>(invoke).unwrap()
    }
}

impl Debug for Function {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct(type_name::<Self>())
            .field("proto_function", &self.proto_function)
            .field("environment", &type_name::<Environment>())
            .finish()
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct TypeMismatch {
    type_name: &'static str,
    type_id: TypeId,
    expected_type_id: TypeId,
}
