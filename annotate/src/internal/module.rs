use super::function::Functions;
use crate::__private::ProtoEnvironment;
use crate::{Attribute, Attributes, Module, Path};
use core::slice::Iter;

#[derive(Debug, Clone)]
pub struct ProtoModule {
    pub(crate) name: &'static str,
    pub(crate) path: Path,
    pub(crate) submodules: &'static [usize],
    pub(crate) functions: &'static [usize],
    pub(crate) _parent_module: Option<usize>,
    pub(crate) attributes: unsafe fn() -> &'static [Attribute],
}

impl Module {
    pub(crate) fn functions(&self) -> Functions {
        Functions::new(self.environment, self.proto_module.functions)
    }

    pub(crate) fn modules(&self) -> Modules {
        Modules::new(self.environment, self.proto_module.submodules)
    }

    pub(crate) fn attributes(&self) -> Attributes {
        Attributes::new(unsafe { (self.proto_module.attributes)() })
    }
}

pub(crate) struct Modules {
    iterator: Iter<'static, usize>,
    environment: &'static ProtoEnvironment,
}

impl Modules {
    pub(crate) fn new(environment: &'static ProtoEnvironment, indices: &'static [usize]) -> Self {
        Self {
            iterator: indices.iter(),
            environment,
        }
    }
}

impl Iterator for Modules {
    type Item = Module;

    fn next(&mut self) -> Option<Self::Item> {
        self.iterator
            .next()
            .map(|index| self.environment.get_module(*index))
    }
}
