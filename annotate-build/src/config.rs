use std::cell::RefCell;
use std::collections::HashMap;
use std::io;
use std::rc::Rc;

use crate::visitor;

#[derive(Debug, PartialEq, serde::Deserialize)]
struct ConfigSpec {
    schema_version: u32,
    #[serde(default)]
    functions: Vec<FunctionSpec>,
    #[serde(default)]
    modules: Vec<ModuleSpec>,
}

#[derive(Debug, PartialEq, serde::Deserialize)]
struct FunctionSpec {
    pragma: String,
}

#[derive(Debug, PartialEq, serde::Deserialize)]
struct ModuleSpec {
    pragma: String,
    #[serde(default)]
    derives: Vec<ModuleDeriveSpec>,
}

#[derive(Debug, PartialEq, serde::Deserialize)]
pub(crate) struct ModuleDeriveSpec {
    pub(crate) name: Option<String>,
    #[serde(default)]
    pub(crate) modules: Vec<ModuleDeriveSpec>,
    #[serde(default)]
    pub(crate) functions: Vec<String>,
}

impl TryFrom<&str> for ConfigSpec {
    type Error = serde_json::Error;

    fn try_from(spec: &str) -> Result<Self, Self::Error> {
        let spec: Self = serde_json::from_str(spec)?;
        if spec.schema_version != crate::SCHEMA_VERSION {
            return Err(serde_json::Error::io(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "schema_version {} does not match supported version {}",
                    spec.schema_version,
                    crate::SCHEMA_VERSION
                ),
            )));
        }

        Ok(spec)
    }
}

pub(crate) fn build_config_from_json_spec(spec: &str) -> Result<BuildConfig, serde_json::Error> {
    let spec = ConfigSpec::try_from(spec)?;
    Ok(BuildConfig::from_spec(&spec))
}

#[derive(Default, Debug)]
pub struct BuildConfig {
    pub(crate) pragmas: Vec<String>,
    pub(crate) derives: Vec<visitor::CustomDerive>,
    pub(crate) module_derives: HashMap<String, Vec<visitor::CustomModuleDerive>>,
}

impl BuildConfig {
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

    fn from_spec(spec: &ConfigSpec) -> Self {
        let mut config = BuildConfig::default();

        for function in &spec.functions {
            push_unique_pragma(&mut config.pragmas, &function.pragma);
        }

        for module in &spec.modules {
            push_unique_pragma(&mut config.pragmas, &module.pragma);
            config.module_derives.insert(
                module.pragma.clone(),
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
        concat!(
            "{",
            "\"schema_version\":",
            $crate::__custom_schema_version!(),
            ",",
            "\"functions\":[",
            $crate::__custom_join!($($functions),*),
            "],",
            "\"modules\":[",
            $crate::__custom_join!($($modules),*),
            "]",
            "}"
        )
    };
    (@parse [ $($functions:expr,)* ] [ $($modules:expr,)* ] fn pragma $pragma:literal, $($rest:tt)*) => {
        $crate::__custom_config!(
            @parse
            [
                $($functions,)*
                concat!("{", "\"pragma\":", $crate::__custom_json_string_literal!($pragma), "}"),
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
                concat!("{", "\"pragma\":", $crate::__custom_json_string_literal!($pragma), "}"),
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
                concat!(
                    "{",
                    "\"pragma\":",
                    $crate::__custom_json_string_literal!($pragma),
                    ",",
                    "\"derives\":[",
                    $crate::__custom_module_derives!(@parse [] $($body)*),
                    "]",
                    "}"
                ),
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
                concat!(
                    "{",
                    "\"pragma\":",
                    $crate::__custom_json_string_literal!($pragma),
                    ",",
                    "\"derives\":[",
                    $crate::__custom_module_derives!(@parse [] $($body)*),
                    "]",
                    "}"
                ),
            ]
        )
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __custom_join {
    () => {
        ""
    };
    ($value:expr) => {
        $value
    };
    ($first:expr, $($rest:expr),+ $(,)?) => {
        concat!($first, ",", $crate::__custom_join!($($rest),+))
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __custom_json_string_literal {
    ($value:literal) => {
        concat!("\"", $value, "\"")
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __custom_schema_version {
    () => {
        "1"
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __custom_module_derives {
    (@parse [ $($derives:expr,)* ]) => {
        $crate::__custom_join!($($derives),*)
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
        concat!(
            "{",
            "\"functions\":[",
            $crate::__custom_join!($($functions),*),
            "],",
            "\"modules\":[",
            $crate::__custom_join!($($modules),*),
            "]",
            "}"
        )
    };
    (@named_ident $name:ident [ $($functions:expr,)* ] [ $($modules:expr,)* ]) => {
        concat!(
            "{",
            "\"name\":\"",
            stringify!($name),
            "\",",
            "\"functions\":[",
            $crate::__custom_join!($($functions),*),
            "],",
            "\"modules\":[",
            $crate::__custom_join!($($modules),*),
            "]",
            "}"
        )
    };
    (@named $name:literal [ $($functions:expr,)* ] [ $($modules:expr,)* ]) => {
        concat!(
            "{",
            "\"name\":",
            $crate::__custom_json_string_literal!($name),
            ",",
            "\"functions\":[",
            $crate::__custom_join!($($functions),*),
            "],",
            "\"modules\":[",
            $crate::__custom_join!($($modules),*),
            "]",
            "}"
        )
    };
    (@unnamed [ $($functions:expr,)* ] [ $($modules:expr,)* ] fn $function:ident, $($rest:tt)*) => {
        $crate::__custom_module_derive_spec!(
            @unnamed
            [ $($functions,)* concat!("\"", stringify!($function), "\""), ]
            [ $($modules,)* ]
            $($rest)*
        )
    };
    (@unnamed [ $($functions:expr,)* ] [ $($modules:expr,)* ] fn $function:ident) => {
        $crate::__custom_module_derive_spec!(
            @unnamed
            [ $($functions,)* concat!("\"", stringify!($function), "\""), ]
            [ $($modules,)* ]
        )
    };
    (@named $name:literal [ $($functions:expr,)* ] [ $($modules:expr,)* ] fn $function:ident, $($rest:tt)*) => {
        $crate::__custom_module_derive_spec!(
            @named
            $name
            [ $($functions,)* concat!("\"", stringify!($function), "\""), ]
            [ $($modules,)* ]
            $($rest)*
        )
    };
    (@named $name:literal [ $($functions:expr,)* ] [ $($modules:expr,)* ] fn $function:ident) => {
        $crate::__custom_module_derive_spec!(
            @named
            $name
            [ $($functions,)* concat!("\"", stringify!($function), "\""), ]
            [ $($modules,)* ]
        )
    };
    (@named_ident $name:ident [ $($functions:expr,)* ] [ $($modules:expr,)* ] fn $function:ident, $($rest:tt)*) => {
        $crate::__custom_module_derive_spec!(
            @named_ident
            $name
            [ $($functions,)* concat!("\"", stringify!($function), "\""), ]
            [ $($modules,)* ]
            $($rest)*
        )
    };
    (@named_ident $name:ident [ $($functions:expr,)* ] [ $($modules:expr,)* ] fn $function:ident) => {
        $crate::__custom_module_derive_spec!(
            @named_ident
            $name
            [ $($functions,)* concat!("\"", stringify!($function), "\""), ]
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

    fn config_spec() -> ConfigSpec {
        ConfigSpec {
            schema_version: crate::SCHEMA_VERSION,
            functions: vec![FunctionSpec {
                pragma: "command".to_string(),
            }],
            modules: vec![ModuleSpec {
                pragma: "tooling".to_string(),
                derives: vec![ModuleDeriveSpec {
                    name: None,
                    functions: vec!["bootstrap".to_string(), "init".to_string()],
                    modules: vec![ModuleDeriveSpec {
                        name: Some("nested".to_string()),
                        functions: vec!["run".to_string()],
                        modules: vec![],
                    }],
                }],
            }],
        }
    }

    const CONFIG_SPEC_USING_MACROS: &str = custom! {
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

    const PHLOW_SPEC_MACROS: &str = custom! {
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

    fn phlow_spec() -> ConfigSpec {
        ConfigSpec {
            schema_version: crate::SCHEMA_VERSION,
            functions: vec![FunctionSpec {
                pragma: "view".to_string(),
            }],
            modules: vec![ModuleSpec {
                pragma: "extensions".to_string(),
                derives: vec![ModuleDeriveSpec {
                    name: Some("__utilities".to_string()),
                    functions: vec![
                        "phlow_to_string".to_string(),
                        "phlow_type_name".to_string(),
                        "phlow_create_view".to_string(),
                        "phlow_defining_methods".to_string(),
                    ],
                    modules: vec![],
                }],
            }],
        }
    }

    const PHLOW_SPEC_JSON: &str = r#"
        {
            "schema_version": 1,
            "functions": [ { "pragma": "view" } ],
            "modules": [ {
                "pragma": "extensions",
                "derives": [
                    {
                        "name": "__utilities",
                        "functions": [
                            "phlow_to_string",
                            "phlow_type_name",
                            "phlow_create_view",
                            "phlow_defining_methods"
                        ]
                    }
                ] } ]
        }
    "#;

    #[test]
    fn build_config_from_spec_collects_pragmas_and_module_derives() {
        let spec = config_spec();
        let config = BuildConfig::from_spec(&spec);

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
        let spec = ConfigSpec::try_from(CONFIG_SPEC_USING_MACROS).unwrap();
        assert_eq!(config_spec(), spec);
    }

    #[test]
    fn phlow_macro_spec_matches_manual_spec() {
        let spec = ConfigSpec::try_from(PHLOW_SPEC_MACROS).unwrap();
        assert_eq!(phlow_spec(), spec);
    }

    #[test]
    fn phlow_json_string_deserializes_as_manual_spec() {
        let spec = ConfigSpec::try_from(PHLOW_SPEC_JSON).unwrap();
        assert_eq!(phlow_spec(), spec);
    }

    #[test]
    fn json_schema_version_must_match() {
        let error = ConfigSpec::try_from(
            r#"
                {
                    "schema_version": 999,
                    "functions": []
                }
            "#,
        )
        .unwrap_err();

        assert_eq!(
            error.to_string(),
            format!(
                "schema_version 999 does not match supported version {}",
                crate::SCHEMA_VERSION
            )
        );
    }

    #[test]
    fn build_config_merge_combines_specs() {
        const SECOND_SPEC: &str = custom! {
            fn pragma "inspect",
            mod pragma "extensions" {
                derive {
                    mod diagnostics {
                        fn collect_warnings,
                    }
                }
            }
        };

        let first_spec = config_spec();
        let second_spec = ConfigSpec::try_from(SECOND_SPEC).unwrap();
        let mut config = BuildConfig::from_spec(&first_spec);
        config.merge(BuildConfig::from_spec(&second_spec));

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

    #[test]
    fn macro_schema_version_matches_public_constant() {
        assert_eq!(
            crate::SCHEMA_VERSION.to_string(),
            crate::__custom_schema_version!()
        );
    }
}
