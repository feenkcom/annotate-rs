use core::slice::Iter;

use crate::{Attribute, Module, Path};

use super::environment::ProtoEnvironment;

#[derive(Debug, Clone)]
pub struct ProtoModule {
    pub(crate) name: &'static str,
    pub(crate) path: Path,
    pub(crate) submodules: &'static [usize],
    pub(crate) functions: &'static [usize],
    pub(crate) _parent_module: Option<usize>,
    pub(crate) attributes: unsafe fn() -> &'static [Attribute],
}

pub(crate) struct Submodules {
    iterator: Iter<'static, usize>,
    environment: &'static ProtoEnvironment,
}

impl Submodules {
    pub(crate) fn new(environment: &'static ProtoEnvironment, indices: &'static [usize]) -> Self {
        Self {
            iterator: indices.iter(),
            environment,
        }
    }
}

impl Iterator for Submodules {
    type Item = Module;

    fn next(&mut self) -> Option<Self::Item> {
        self.iterator
            .next()
            .map(|index| self.environment.get_module(*index))
    }
}
