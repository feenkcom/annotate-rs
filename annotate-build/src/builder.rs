use cargo_toml::{Manifest, Value};
use std::path::{Path, PathBuf};

use crate::parser::Parser;
use crate::visitor::CustomDerive;

fn export_target(
    crate_root: &Path,
    pragmas: &[String],
    derives: &[CustomDerive],
    target_name: &str,
    target_path: impl Into<PathBuf>,
) {
    Parser::custom(target_name, crate_root.to_path_buf(), target_path)
        .with_pragmas(pragmas)
        .with_derives(derives)
        .export();
}

pub(crate) fn build_manifest<P, T: ToString>(
    pragmas: P,
    derives: &[CustomDerive],
    manifest_path: impl Into<PathBuf>,
    crate_root: impl Into<PathBuf>,
) where
    P: IntoIterator<Item = T>,
{
    let package_name = std::env::var("CARGO_PKG_NAME").unwrap().replace("-", "_");
    let pragmas = pragmas
        .into_iter()
        .map(|item| item.to_string())
        .collect::<Vec<String>>();

    let manifest_path = manifest_path.into();
    let crate_root = crate_root.into();
    let manifest: Manifest<Value> = Manifest::from_path_with_metadata(&manifest_path).unwrap();
    let lib = manifest.lib;
    let lib_path = lib
        .as_ref()
        .and_then(|target| target.path.clone())
        .unwrap_or_else(|| "src/lib.rs".to_string());
    let lib_name = lib
        .and_then(|target| target.name)
        .unwrap_or_else(|| package_name.clone());

    if manifest_path
        .parent()
        .unwrap()
        .join(lib_path.as_str())
        .is_file()
    {
        export_target(
            crate_root.as_path(),
            pragmas.as_slice(),
            derives,
            lib_name.as_str(),
            lib_path,
        );
    }
}
