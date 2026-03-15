pub(crate) mod environment;
pub(crate) mod function;
pub(crate) mod module;

pub(crate) const fn const_bytes_equal(lhs: &[u8], rhs: &[u8]) -> bool {
    if lhs.len() != rhs.len() {
        return false;
    }

    let mut index = 0;
    while index < lhs.len() {
        if lhs[index] != rhs[index] {
            return false;
        }
        index += 1;
    }

    true
}

pub(crate) const fn const_str_equal(lhs: &str, rhs: &str) -> bool {
    const_bytes_equal(lhs.as_bytes(), rhs.as_bytes())
}

#[doc(hidden)]
pub mod __private {
    pub use super::environment::ProtoEnvironment;
    pub use super::function::{AnyReturn, FunctionPointer, ProtoFunction};
    pub use super::module::ProtoModule;

    pub const fn path(segments: &'static [&'static str]) -> crate::Path {
        crate::Path(segments)
    }

    pub const fn attribute(name: &'static str, value: crate::Value) -> crate::Attribute {
        crate::Attribute { name, value }
    }

    pub const fn ty<T: 'static>() -> crate::Type {
        crate::Type {
            type_name_fn: core::any::type_name::<T>,
            type_id: core::any::TypeId::of::<T>(),
        }
    }

    pub const fn proto_environment(
        modules: &'static [ProtoModule],
        functions: &'static [ProtoFunction],
    ) -> ProtoEnvironment {
        ProtoEnvironment { modules, functions }
    }

    pub const fn proto_function(
        name: &'static str,
        path: crate::Path,
        module: Option<usize>,
        attributes: unsafe fn() -> &'static [crate::Attribute],
        function: unsafe fn() -> FunctionPointer,
    ) -> ProtoFunction {
        ProtoFunction {
            name,
            path,
            module,
            attributes,
            function,
        }
    }

    pub const fn proto_module(
        name: &'static str,
        path: crate::Path,
        submodules: &'static [usize],
        functions: &'static [usize],
        parent_module: Option<usize>,
        attributes: unsafe fn() -> &'static [crate::Attribute],
    ) -> ProtoModule {
        ProtoModule {
            name,
            path,
            _submodules: submodules,
            functions,
            _parent_module: parent_module,
            attributes,
        }
    }

    pub const fn environment(
        modules: &'static [crate::Module],
        functions: &'static [crate::Function],
    ) -> crate::Environment {
        crate::Environment { modules, functions }
    }

    pub const fn function(
        environment: &'static ProtoEnvironment,
        proto_function: &'static ProtoFunction,
    ) -> crate::Function {
        crate::Function {
            environment,
            proto_function,
        }
    }

    pub const fn module(
        environment: &'static ProtoEnvironment,
        proto_module: &'static ProtoModule,
    ) -> crate::Module {
        crate::Module {
            environment,
            proto_module,
        }
    }

    pub fn function_pointer(
        raw: &'static dyn core::any::Any,
        any_return: Option<fn() -> AnyReturn>,
    ) -> FunctionPointer {
        FunctionPointer {
            raw,
            _any_return: any_return,
        }
    }

    pub fn any_return<T: Send + Sync + 'static>(value: T) -> AnyReturn {
        #[cfg(feature = "function-call")]
        {
            AnyReturn::Value(alloc::boxed::Box::new(value))
        }
        #[cfg(not(feature = "function-call"))]
        {
            let _ = value;
            AnyReturn::Unsupported
        }
    }
}
