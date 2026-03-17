#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use annotate_derive::{environment, pragma};
pub use attribute::{Attribute, Attributes, Type, Value};
pub use environment::Environment;
pub use function::{Function, TypeMismatch};
pub use module::Module;
pub use path::Path;

mod attribute;
mod environment;
mod function;
mod internal;
mod module;
mod path;

#[cfg(feature = "global-environment")]
mod global_environment;

#[cfg(feature = "global-environment")]
pub use global_environment::*;

#[doc(hidden)]
pub use internal::__private;
