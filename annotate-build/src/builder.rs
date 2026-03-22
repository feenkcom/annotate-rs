use cargo_toml::{Manifest, Product, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::parser::Parser;
use crate::visitor::{CustomDerive, CustomModuleDerive};

struct ExportContext<'a> {
    manifest_dir: &'a Path,
    crate_root: &'a Path,
    pragmas: &'a [String],
    derives: &'a [CustomDerive],
    module_derives: &'a HashMap<String, Vec<CustomModuleDerive>>,
}

struct ProductDefaults<'a> {
    name: &'a str,
    path: &'a str,
}

fn export_target(
    crate_root: &Path,
    pragmas: &[String],
    derives: &[CustomDerive],
    module_derives: &HashMap<String, Vec<CustomModuleDerive>>,
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
        .with_module_derives(module_derives.clone())
        .export_to(&output_path);
}

fn export_product(
    cx: &ExportContext<'_>,
    defaults: ProductDefaults<'_>,
    product: Option<&Product>,
) {
    let target_path = product
        .and_then(|target| target.path.clone())
        .unwrap_or_else(|| defaults.path.to_string());
    let target_name = product
        .and_then(|target| target.name.clone())
        .unwrap_or_else(|| defaults.name.to_string());

    if cx.manifest_dir.join(&target_path).is_file() {
        export_target(
            cx.crate_root,
            cx.pragmas,
            cx.derives,
            cx.module_derives,
            target_name.as_str(),
            target_path,
        );
    }
}

fn export_products(cx: &ExportContext<'_>, products: &[Product]) {
    for product in products {
        if let Some(path) = product.path.as_ref()
            && cx.manifest_dir.join(path).is_file()
        {
            let target_name = product
                .name
                .clone()
                .unwrap_or_else(|| default_target_name(path));
            export_target(
                cx.crate_root,
                cx.pragmas,
                cx.derives,
                cx.module_derives,
                target_name.as_str(),
                path,
            );
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
    module_derives: HashMap<String, Vec<CustomModuleDerive>>,
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
    let export_context = ExportContext {
        manifest_dir,
        crate_root: crate_root.as_path(),
        pragmas: pragmas.as_slice(),
        derives,
        module_derives: &module_derives,
    };

    export_product(
        &export_context,
        ProductDefaults {
            name: package_name.as_str(),
            path: "src/lib.rs",
        },
        manifest.lib.as_ref(),
    );

    export_products(&export_context, manifest.bin.as_slice());
    export_products(&export_context, manifest.example.as_slice());
    export_products(&export_context, manifest.test.as_slice());
    export_products(&export_context, manifest.bench.as_slice());
}
