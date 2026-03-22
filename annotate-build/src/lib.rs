use std::path::PathBuf;

pub use config::{BuildConfig, ModuleDeriveBuilder};

pub const SCHEMA_VERSION: u32 = 1;

pub(crate) use function::*;
pub(crate) use module::*;
pub(crate) use path::*;
pub(crate) use render::*;

mod builder;
mod config;
mod environment;
mod function;
mod module;
mod parser;
mod path;
mod render;
mod visitor;

/// Scan the current crate for `#[pragma(...)]` annotations and export the generated environment.
pub fn build() {
    build_with(|_| {});
}

/// Scan the current crate and export the generated environment from multiple JSON configuration
/// specs.
///
/// Each spec must be a JSON string with the following shape:
///
/// ```json
/// {
///   "schema_version": 1,
///   "functions": [
///     { "pragma": "command" }
///   ],
///   "modules": [
///     {
///       "pragma": "plugin",
///       "derives": [
///         {
///           "name": "logging",
///           "functions": [
///             "info",
///             "warn",
///             "error"
///           ],
///           "modules": [
///             {
///               "name": "http",
///               "functions": [
///                 "request_started",
///                 "request_finished"
///               ],
///               "modules": []
///             }
///           ]
///         }
///       ]
///     }
///   ]
/// }
/// ```
///
/// Fields:
/// - `schema_version`: Must match [`SCHEMA_VERSION`]. This makes format changes explicit and lets
///   builds fail early on incompatible config.
/// - `functions`: Additional attribute names that should be treated like `#[pragma(...)]` on
///   functions.
/// - `modules`: Additional attribute names that should be treated like `#[pragma(...)]` on
///   modules, together with optional nested derive configuration.
/// - `derives[*].name`: Optional derive/module name.
/// - `derives[*].functions`: Function names to expose under that derive node.
/// - `derives[*].modules`: Nested derive modules with the same structure.
pub fn build_with_specs(specs: impl IntoIterator<Item: AsRef<str>>) {
    let mut config = BuildConfig::default();

    for spec in specs {
        let spec_config = config::build_config_from_json_spec(spec.as_ref())
            .expect("config spec string must be valid JSON with a supported schema version");
        config.merge(spec_config);
    }

    build_from_config(config);
}

/// Scan the current crate and export the generated environment using a small configuration DSL.
pub fn build_with(configure: impl FnOnce(&mut BuildConfig)) {
    let mut config = BuildConfig::default();
    configure(&mut config);

    build_from_config(config);
}

fn build_from_config(config: BuildConfig) {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());

    builder::build_manifest(
        config.pragmas,
        &config.derives,
        config.module_derives,
        manifest_dir.join("Cargo.toml"),
        manifest_dir.file_name().unwrap(),
    )
}
