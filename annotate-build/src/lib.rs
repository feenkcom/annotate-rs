use std::path::PathBuf;

pub(crate) use function::*;
pub(crate) use module::*;
pub(crate) use path::*;
pub(crate) use render::*;

mod builder;
mod environment;
mod function;
mod module;
mod parser;
mod path;
mod render;
mod visitor;

#[derive(Default)]
pub struct BuildConfig {
    pragmas: Vec<String>,
    derives: Vec<visitor::CustomDerive>,
}

impl BuildConfig {
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
}

/// Scan the current crate for `#[pragma(...)]` annotations and export the generated environment.
pub fn build() {
    build_with(|_| {});
}

/// Scan the current crate and export the generated environment using a small configuration DSL.
pub fn build_with(configure: impl FnOnce(&mut BuildConfig)) {
    let mut config = BuildConfig::default();
    configure(&mut config);

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());

    builder::build_manifest(
        config.pragmas,
        &config.derives,
        manifest_dir.join("Cargo.toml"),
        manifest_dir.file_name().unwrap(),
    )
}
