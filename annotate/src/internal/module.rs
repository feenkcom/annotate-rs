use crate::{Attribute, Attributes, Module, Path};

use super::function::Functions;

#[derive(Debug, Clone)]
pub struct ProtoModule {
    pub(crate) name: &'static str,
    pub(crate) path: Path,
    pub(crate) _submodules: &'static [usize],
    pub(crate) functions: &'static [usize],
    pub(crate) _parent_module: Option<usize>,
    pub(crate) attributes: unsafe fn() -> &'static [Attribute],
}

impl Module {
    pub(crate) fn functions(&self) -> Functions {
        Functions::new(self.environment, self.proto_module.functions)
    }

    pub(crate) fn attributes(&self) -> Attributes {
        Attributes::new(unsafe { (self.proto_module.attributes)() })
    }
}

