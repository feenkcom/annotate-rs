use std::path::PathBuf;

pub use config::{
    BuildConfig, ConfigSpec, FunctionSpec, ModuleDeriveBuilder, ModuleDeriveSpec, ModuleSpec,
};

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

/// Scan the current crate and export the generated environment from multiple configuration specs.
pub fn build_with_specs<'a>(specs: impl IntoIterator<Item = &'a ConfigSpec>) {
    let mut config = BuildConfig::default();

    for spec in specs {
        config.merge(BuildConfig::from_spec(spec));
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
