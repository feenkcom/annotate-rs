use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::visitor;

#[derive(Debug, PartialEq)]
pub struct ConfigSpec {
    pub functions: &'static [FunctionSpec],
    pub modules: &'static [ModuleSpec],
}

#[derive(Debug, PartialEq)]
pub struct FunctionSpec {
    pub pragma: &'static str,
}

#[derive(Debug, PartialEq)]
pub struct ModuleSpec {
    pub pragma: &'static str,
    pub derives: &'static [ModuleDeriveSpec],
}

#[derive(Debug, PartialEq)]
pub struct ModuleDeriveSpec {
    pub name: Option<&'static str>,
    pub modules: &'static [ModuleDeriveSpec],
    pub functions: &'static [&'static str],
}

#[derive(Default, Debug)]
pub struct BuildConfig {
    pub(crate) pragmas: Vec<String>,
    pub(crate) derives: Vec<visitor::CustomDerive>,
    pub(crate) module_derives: HashMap<String, Vec<visitor::CustomModuleDerive>>,
}

impl BuildConfig {
    pub fn from_spec(spec: &ConfigSpec) -> Self {
        spec.into()
    }

    pub(crate) fn merge(&mut self, other: Self) {
        for pragma in other.pragmas {
            push_unique_pragma(&mut self.pragmas, pragma.as_str());
        }

        self.derives.extend(other.derives);

        for (pragma, derives) in other.module_derives {
            self.module_derives
                .entry(pragma)
                .or_default()
                .extend(derives);
        }
    }

    /// Register an additional attribute name that should be treated like `#[pragma(...)]`.
    pub fn pragma(&mut self, pragma: impl Into<String>) -> &mut Self {
        self.pragmas.push(pragma.into());
        self
    }

    /// Register a custom derive name and its mustache template expansion.
    pub fn derive(&mut self, name: impl Into<String>, template: impl AsRef<str>) -> &mut Self {
        self.derives
            .push(visitor::CustomDerive::new(name, template.as_ref()));
        self
    }

    pub fn module_derive(
        &mut self,
        extension: impl Into<String>,
        configure: impl FnOnce(&mut ModuleDeriveBuilder),
    ) -> &mut Self {
        let extension = extension.into();
        let existing_derives = self
            .module_derives
            .get(&extension)
            .cloned()
            .unwrap_or_default();

        let mut builder = ModuleDeriveBuilder::new(existing_derives);
        configure(&mut builder);

        self.module_derives
            .insert(extension, builder.derives.borrow().clone());

        self
    }
}

impl From<&ConfigSpec> for BuildConfig {
    fn from(spec: &ConfigSpec) -> Self {
        let mut config = BuildConfig::default();

        for function in spec.functions {
            push_unique_pragma(&mut config.pragmas, function.pragma);
        }

        for module in spec.modules {
            push_unique_pragma(&mut config.pragmas, module.pragma);
            config.module_derives.insert(
                module.pragma.to_string(),
                module
                    .derives
                    .iter()
                    .map(custom_module_derive_from_spec)
                    .collect(),
            );
        }

        config
    }
}

#[derive(Default, Clone)]
pub struct ModuleDeriveBuilder {
    derives: Rc<RefCell<Vec<visitor::CustomModuleDerive>>>,
    current_derive_path: Vec<usize>,
}

impl ModuleDeriveBuilder {
    fn new(existing_derives: Vec<visitor::CustomModuleDerive>) -> Self {
        Self {
            derives: Rc::new(RefCell::new(existing_derives)),
            current_derive_path: vec![],
        }
    }

    pub fn module(&mut self, module: impl Into<String>) -> Self {
        let mut clone = self.clone();

        if clone.current_derive_path.is_empty() {
            let index = clone.derives.borrow().len();
            clone
                .derives
                .borrow_mut()
                .push(visitor::CustomModuleDerive::new(module.into()));
            clone.current_derive_path.push(index);
        } else {
            let mut index = 0;
            clone.with_derive_at_path(|derive| {
                index = derive.add_module(module.into());
            });
            clone.current_derive_path.push(index);
        }

        clone
    }

    pub fn functions(&mut self, functions: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let clone = self.clone();

        clone.with_derive_at_path(|derive| {
            for each in functions {
                derive.add_function(each.into());
            }
        });

        clone
    }

    fn with_derive_at_path(&self, f: impl FnOnce(&mut visitor::CustomModuleDerive)) {
        if self.derives.borrow().is_empty() {
            self.derives
                .borrow_mut()
                .push(visitor::CustomModuleDerive::default());
        }

        let mut borrow = self.derives.borrow_mut();
        if self.current_derive_path.is_empty() {
            let last_derive = borrow
                .last_mut()
                .unwrap()
                .derive_at_mut(&self.current_derive_path);

            f(last_derive)
        } else {
            let index = *self.current_derive_path.first().unwrap();
            let first_derive = borrow.get_mut(index).unwrap();
            f(first_derive.derive_at_mut(&self.current_derive_path[1..]));
        }
    }
}

fn push_unique_pragma(pragmas: &mut Vec<String>, pragma: &str) {
    if pragmas.iter().all(|existing| existing != pragma) {
        pragmas.push(pragma.to_string());
    }
}

fn custom_module_derive_from_spec(spec: &ModuleDeriveSpec) -> visitor::CustomModuleDerive {
    visitor::CustomModuleDerive::from_spec(spec)
}

#[macro_export]
macro_rules! custom {
    ($($tokens:tt)*) => {
        $crate::__custom_config!(@parse [] [] $($tokens)*)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __custom_config {
    (@parse [ $($functions:expr,)* ] [ $($modules:expr,)* ]) => {
        $crate::ConfigSpec {
            functions: &[ $($functions,)* ],
            modules: &[ $($modules,)* ],
        }
    };
    (@parse [ $($functions:expr,)* ] [ $($modules:expr,)* ] fn pragma $pragma:literal, $($rest:tt)*) => {
        $crate::__custom_config!(
            @parse
            [
                $($functions,)*
                $crate::FunctionSpec { pragma: $pragma },
            ]
            [ $($modules,)* ]
            $($rest)*
        )
    };
    (@parse [ $($functions:expr,)* ] [ $($modules:expr,)* ] fn pragma $pragma:literal) => {
        $crate::__custom_config!(
            @parse
            [
                $($functions,)*
                $crate::FunctionSpec { pragma: $pragma },
            ]
            [ $($modules,)* ]
        )
    };
    (@parse [ $($functions:expr,)* ] [ $($modules:expr,)* ] mod pragma $pragma:literal { $($body:tt)* }, $($rest:tt)*) => {
        $crate::__custom_config!(
            @parse
            [ $($functions,)* ]
            [
                $($modules,)*
                $crate::ModuleSpec {
                    pragma: $pragma,
                    derives: &$crate::__custom_module_derives!(@parse [] $($body)*),
                },
            ]
            $($rest)*
        )
    };
    (@parse [ $($functions:expr,)* ] [ $($modules:expr,)* ] mod pragma $pragma:literal { $($body:tt)* }) => {
        $crate::__custom_config!(
            @parse
            [ $($functions,)* ]
            [
                $($modules,)*
                $crate::ModuleSpec {
                    pragma: $pragma,
                    derives: &$crate::__custom_module_derives!(@parse [] $($body)*),
                },
            ]
        )
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __custom_module_derives {
    (@parse [ $($derives:expr,)* ]) => {
        [ $($derives,)* ]
    };
    (@parse [ $($derives:expr,)* ] derive { mod $name:ident { $($body:tt)* } }, $($rest:tt)*) => {
        $crate::__custom_module_derives!(
            @parse
            [
                $($derives,)*
                $crate::__custom_module_derive_spec!(@named_ident $name [] [] $($body)*),
            ]
            $($rest)*
        )
    };
    (@parse [ $($derives:expr,)* ] derive { mod $name:ident { $($body:tt)* } }) => {
        $crate::__custom_module_derives!(
            @parse
            [
                $($derives,)*
                $crate::__custom_module_derive_spec!(@named_ident $name [] [] $($body)*),
            ]
        )
    };
    (@parse [ $($derives:expr,)* ] derive { $($body:tt)* }, $($rest:tt)*) => {
        $crate::__custom_module_derives!(
            @parse
            [
                $($derives,)*
                $crate::__custom_module_derive_spec!(@unnamed [] [] $($body)*),
            ]
            $($rest)*
        )
    };
    (@parse [ $($derives:expr,)* ] derive { $($body:tt)* }) => {
        $crate::__custom_module_derives!(
            @parse
            [
                $($derives,)*
                $crate::__custom_module_derive_spec!(@unnamed [] [] $($body)*),
            ]
        )
    };
    (@parse [ $($derives:expr,)* ] derive $name:literal { $($body:tt)* }, $($rest:tt)*) => {
        $crate::__custom_module_derives!(
            @parse
            [
                $($derives,)*
                $crate::__custom_module_derive_spec!(@named $name [] [] $($body)*),
            ]
            $($rest)*
        )
    };
    (@parse [ $($derives:expr,)* ] derive $name:literal { $($body:tt)* }) => {
        $crate::__custom_module_derives!(
            @parse
            [
                $($derives,)*
                $crate::__custom_module_derive_spec!(@named $name [] [] $($body)*),
            ]
        )
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __custom_module_derive_spec {
    (@unnamed [ $($functions:expr,)* ] [ $($modules:expr,)* ]) => {
        $crate::ModuleDeriveSpec {
            name: None,
            functions: &[ $($functions,)* ],
            modules: &[ $($modules,)* ],
        }
    };
    (@named_ident $name:ident [ $($functions:expr,)* ] [ $($modules:expr,)* ]) => {
        $crate::ModuleDeriveSpec {
            name: Some(stringify!($name)),
            functions: &[ $($functions,)* ],
            modules: &[ $($modules,)* ],
        }
    };
    (@named $name:literal [ $($functions:expr,)* ] [ $($modules:expr,)* ]) => {
        $crate::ModuleDeriveSpec {
            name: Some($name),
            functions: &[ $($functions,)* ],
            modules: &[ $($modules,)* ],
        }
    };
    (@unnamed [ $($functions:expr,)* ] [ $($modules:expr,)* ] fn $function:ident, $($rest:tt)*) => {
        $crate::__custom_module_derive_spec!(
            @unnamed
            [ $($functions,)* stringify!($function), ]
            [ $($modules,)* ]
            $($rest)*
        )
    };
    (@unnamed [ $($functions:expr,)* ] [ $($modules:expr,)* ] fn $function:ident) => {
        $crate::__custom_module_derive_spec!(
            @unnamed
            [ $($functions,)* stringify!($function), ]
            [ $($modules,)* ]
        )
    };
    (@named $name:literal [ $($functions:expr,)* ] [ $($modules:expr,)* ] fn $function:ident, $($rest:tt)*) => {
        $crate::__custom_module_derive_spec!(
            @named
            $name
            [ $($functions,)* stringify!($function), ]
            [ $($modules,)* ]
            $($rest)*
        )
    };
    (@named $name:literal [ $($functions:expr,)* ] [ $($modules:expr,)* ] fn $function:ident) => {
        $crate::__custom_module_derive_spec!(
            @named
            $name
            [ $($functions,)* stringify!($function), ]
            [ $($modules,)* ]
        )
    };
    (@named_ident $name:ident [ $($functions:expr,)* ] [ $($modules:expr,)* ] fn $function:ident, $($rest:tt)*) => {
        $crate::__custom_module_derive_spec!(
            @named_ident
            $name
            [ $($functions,)* stringify!($function), ]
            [ $($modules,)* ]
            $($rest)*
        )
    };
    (@named_ident $name:ident [ $($functions:expr,)* ] [ $($modules:expr,)* ] fn $function:ident) => {
        $crate::__custom_module_derive_spec!(
            @named_ident
            $name
            [ $($functions,)* stringify!($function), ]
            [ $($modules,)* ]
        )
    };
    (@unnamed [ $($functions:expr,)* ] [ $($modules:expr,)* ] mod $name:literal { $($body:tt)* }, $($rest:tt)*) => {
        $crate::__custom_module_derive_spec!(
            @unnamed
            [ $($functions,)* ]
            [
                $($modules,)*
                $crate::__custom_module_derive_spec!(@named $name [] [] $($body)*),
            ]
            $($rest)*
        )
    };
    (@unnamed [ $($functions:expr,)* ] [ $($modules:expr,)* ] mod $name:literal { $($body:tt)* }) => {
        $crate::__custom_module_derive_spec!(
            @unnamed
            [ $($functions,)* ]
            [
                $($modules,)*
                $crate::__custom_module_derive_spec!(@named $name [] [] $($body)*),
            ]
        )
    };
    (@unnamed [ $($functions:expr,)* ] [ $($modules:expr,)* ] mod $name:ident { $($body:tt)* }, $($rest:tt)*) => {
        $crate::__custom_module_derive_spec!(
            @unnamed
            [ $($functions,)* ]
            [
                $($modules,)*
                $crate::__custom_module_derive_spec!(@named_ident $name [] [] $($body)*),
            ]
            $($rest)*
        )
    };
    (@unnamed [ $($functions:expr,)* ] [ $($modules:expr,)* ] mod $name:ident { $($body:tt)* }) => {
        $crate::__custom_module_derive_spec!(
            @unnamed
            [ $($functions,)* ]
            [
                $($modules,)*
                $crate::__custom_module_derive_spec!(@named_ident $name [] [] $($body)*),
            ]
        )
    };
    (@named $name:literal [ $($functions:expr,)* ] [ $($modules:expr,)* ] mod $module_name:ident { $($body:tt)* }, $($rest:tt)*) => {
        $crate::__custom_module_derive_spec!(
            @named
            $name
            [ $($functions,)* ]
            [
                $($modules,)*
                $crate::__custom_module_derive_spec!(@named_ident $module_name [] [] $($body)*),
            ]
            $($rest)*
        )
    };
    (@named $name:literal [ $($functions:expr,)* ] [ $($modules:expr,)* ] mod $module_name:ident { $($body:tt)* }) => {
        $crate::__custom_module_derive_spec!(
            @named
            $name
            [ $($functions,)* ]
            [
                $($modules,)*
                $crate::__custom_module_derive_spec!(@named_ident $module_name [] [] $($body)*),
            ]
        )
    };
    (@named_ident $name:ident [ $($functions:expr,)* ] [ $($modules:expr,)* ] mod $module_name:ident { $($body:tt)* }, $($rest:tt)*) => {
        $crate::__custom_module_derive_spec!(
            @named_ident
            $name
            [ $($functions,)* ]
            [
                $($modules,)*
                $crate::__custom_module_derive_spec!(@named_ident $module_name [] [] $($body)*),
            ]
            $($rest)*
        )
    };
    (@named_ident $name:ident [ $($functions:expr,)* ] [ $($modules:expr,)* ] mod $module_name:ident { $($body:tt)* }) => {
        $crate::__custom_module_derive_spec!(
            @named_ident
            $name
            [ $($functions,)* ]
            [
                $($modules,)*
                $crate::__custom_module_derive_spec!(@named_ident $module_name [] [] $($body)*),
            ]
        )
    };
}

#[cfg(test)]
mod tests {
    use super::{ConfigSpec, FunctionSpec, ModuleDeriveSpec, ModuleSpec};
    use crate::BuildConfig;

    const CONFIG_SPEC: ConfigSpec = ConfigSpec {
        functions: &[FunctionSpec { pragma: "command" }],
        modules: &[ModuleSpec {
            pragma: "tooling",
            derives: &[ModuleDeriveSpec {
                name: None,
                functions: &["bootstrap", "init"],
                modules: &[ModuleDeriveSpec {
                    name: Some("nested"),
                    functions: &["run"],
                    modules: &[],
                }],
            }],
        }],
    };

    const CONFIG_SPEC_USING_MACROS: ConfigSpec = custom! {
        fn pragma "command",
        mod pragma "tooling" {
            derive {
                fn bootstrap,
                fn init,
                mod nested {
                    fn run,
                }
            }
        }
    };

    const PHLOW_SPEC_MACROS: ConfigSpec = custom! {
        fn pragma "view",
        mod pragma "extensions" {
            derive {
                mod __utilities {
                    fn phlow_to_string,
                    fn phlow_type_name,
                    fn phlow_create_view,
                    fn phlow_defining_methods,
                }
            }
        }
    };

    const PHLOW_SPEC: ConfigSpec = ConfigSpec {
        functions: &[FunctionSpec { pragma: "view" }],
        modules: &[ModuleSpec {
            pragma: "extensions",
            derives: &[ModuleDeriveSpec {
                name: Some("__utilities"),
                functions: &[
                    "phlow_to_string",
                    "phlow_type_name",
                    "phlow_create_view",
                    "phlow_defining_methods",
                ],
                modules: &[],
            }],
        }],
    };

    #[test]
    fn build_config_from_spec_collects_pragmas_and_module_derives() {
        let config = BuildConfig::from_spec(&CONFIG_SPEC);

        assert_eq!(config.pragmas, vec!["command", "tooling"]);
        assert!(config.derives.is_empty());

        let tooling_derives = config.module_derives.get("tooling").unwrap();
        assert_eq!(tooling_derives.len(), 1);

        let root_derive = &tooling_derives[0];
        assert_eq!(root_derive.function_names(), ["bootstrap", "init"]);
        assert_eq!(root_derive.modules().len(), 1);

        let nested = &root_derive.modules()[0];
        assert_eq!(nested.name(), Some("nested"));
        assert_eq!(nested.function_names(), ["run"]);
    }

    #[test]
    fn custom_macro_builds_the_same_config_spec() {
        assert_eq!(CONFIG_SPEC, CONFIG_SPEC_USING_MACROS);
    }

    #[test]
    fn phlow_macro_spec_matches_manual_spec() {
        assert_eq!(PHLOW_SPEC, PHLOW_SPEC_MACROS);
    }

    #[test]
    fn build_config_merge_combines_specs() {
        const SECOND_SPEC: ConfigSpec = custom! {
            fn pragma "inspect",
            mod pragma "extensions" {
                derive {
                    mod diagnostics {
                        fn collect_warnings,
                    }
                }
            }
        };

        let mut config = BuildConfig::from_spec(&CONFIG_SPEC);
        config.merge(BuildConfig::from_spec(&SECOND_SPEC));

        assert_eq!(
            config.pragmas,
            vec!["command", "tooling", "inspect", "extensions"]
        );

        let tooling_derives = config.module_derives.get("tooling").unwrap();
        assert_eq!(tooling_derives.len(), 1);

        let extension_derives = config.module_derives.get("extensions").unwrap();
        assert_eq!(extension_derives.len(), 1);
        assert_eq!(extension_derives[0].name(), Some("diagnostics"));
        assert_eq!(extension_derives[0].function_names(), ["collect_warnings"]);
    }
}
