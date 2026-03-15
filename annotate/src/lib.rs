#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use annotate_derive::pragma;
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

#[macro_export]
macro_rules! environment {
    () => {
        #[macro_use]
        extern crate annotate;
        extern crate alloc;

        include!(concat!(env!("OUT_DIR"), "/annotate/", file!()));
        pub const fn environment() -> &'static annotate::Environment {
            &__annotate::ENVIRONMENT
        }
    };
    ($path:path) => {
        mod __annotate {
            use $path;
            include!(concat!(env!("OUT_DIR"), "/annotate/", file!()));
            pub const fn environment() -> &'static annotate::Environment {
                &__annotate::ENVIRONMENT
            }
        }
    };
}
