use alloc::boxed::Box;
use core::any::Any;
use core::slice::Iter;

use crate::{Attribute, Function, Path};

use super::environment::ProtoEnvironment;

#[derive(Debug, Clone)]
pub struct ProtoFunction {
    pub(crate) name: &'static str,
    pub(crate) path: Path,
    pub(crate) module: Option<usize>,
    pub(crate) attributes: unsafe fn() -> &'static [Attribute],
    pub(crate) function: unsafe fn() -> FunctionPointer,
}

pub struct FunctionPointer {
    pub(crate) raw: &'static dyn Any,
    pub(crate) _any_return: Option<fn() -> AnyReturn>,
}

pub enum AnyReturn {
    #[cfg(feature = "function-call")]
    Value(Box<dyn Any + Send + Sync>),
    Unsupported,
}

pub(crate) struct Functions {
    iterator: Iter<'static, usize>,
    environment: &'static ProtoEnvironment,
}

impl Functions {
    pub(crate) fn new(environment: &'static ProtoEnvironment, indices: &'static [usize]) -> Self {
        Self {
            iterator: indices.iter(),
            environment,
        }
    }
}

impl Iterator for Functions {
    type Item = Function;

    fn next(&mut self) -> Option<Self::Item> {
        self.iterator
            .next()
            .map(|index| self.environment.get_function(*index))
    }
}
