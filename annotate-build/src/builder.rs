use cargo_toml::{Manifest, Product, Value};
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
    let target_path = target_path.into();
    let output_path = PathBuf::from("annotate")
        .join(crate_root)
        .join(&target_path);

    Parser::custom(target_name, crate_root.to_path_buf(), &target_path)
        .with_pragmas(pragmas)
        .with_derives(derives)
        .export_to(output_path);
}

fn export_product(
    manifest_dir: &Path,
    crate_root: &Path,
    pragmas: &[String],
    derives: &[CustomDerive],
    default_name: &str,
    default_path: &str,
    product: Option<&Product>,
) {
    let target_path = product
        .and_then(|target| target.path.clone())
        .unwrap_or_else(|| default_path.to_string());
    let target_name = product
        .and_then(|target| target.name.clone())
        .unwrap_or_else(|| default_name.to_string());

    if manifest_dir.join(&target_path).is_file() {
        export_target(
            crate_root,
            pragmas,
            derives,
            target_name.as_str(),
            target_path,
        );
    }
}

fn export_products(
    manifest_dir: &Path,
    crate_root: &Path,
    pragmas: &[String],
    derives: &[CustomDerive],
    products: &[Product],
) {
    for product in products {
        if let Some(path) = product.path.as_ref()
            && manifest_dir.join(path).is_file()
        {
            let target_name = product
                .name
                .clone()
                .unwrap_or_else(|| default_target_name(path));
            export_target(crate_root, pragmas, derives, target_name.as_str(), path);
        }
    }
}

fn default_target_name(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap()
        .replace('-', "_")
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
    let manifest_dir = manifest_path.parent().unwrap();

    export_product(
        manifest_dir,
        crate_root.as_path(),
        pragmas.as_slice(),
        derives,
        package_name.as_str(),
        "src/lib.rs",
        manifest.lib.as_ref(),
    );

    export_products(
        manifest_dir,
        crate_root.as_path(),
        pragmas.as_slice(),
        derives,
        manifest.bin.as_slice(),
    );
    export_products(
        manifest_dir,
        crate_root.as_path(),
        pragmas.as_slice(),
        derives,
        manifest.example.as_slice(),
    );
    export_products(
        manifest_dir,
        crate_root.as_path(),
        pragmas.as_slice(),
        derives,
        manifest.test.as_slice(),
    );
    export_products(
        manifest_dir,
        crate_root.as_path(),
        pragmas.as_slice(),
        derives,
        manifest.bench.as_slice(),
    );
}
