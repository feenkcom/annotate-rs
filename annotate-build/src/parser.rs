use std::fs;
use std::io::Read;
use std::path::PathBuf;

use proc_macro2::Span;
use syn::visit::Visit;

use crate::environment::Environment;
use crate::visitor::{CustomDerive, Visitor};

pub(crate) struct Parser {
    crate_name: String,
    crate_root: PathBuf,
    start_file: PathBuf,
    pragmas: Vec<String>,
    derives: Vec<CustomDerive>,
}

impl Parser {
    pub(crate) fn new() -> Self {
        let root_module = std::env::var("CARGO_PKG_NAME").unwrap().replace("-", "_");
        let crate_root = PathBuf::from(
            PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
                .file_name()
                .unwrap(),
        );

        Self {
            crate_name: root_module,
            crate_root,
            start_file: PathBuf::from("src/lib.rs"),
            pragmas: vec![],
            derives: vec![],
        }
    }

    pub(crate) fn custom(
        crate_name: &str,
        crate_root: impl Into<PathBuf>,
        start_file: impl Into<PathBuf>,
    ) -> Self {
        Self {
            crate_name: crate_name.to_string(),
            crate_root: crate_root.into(),
            start_file: start_file.into(),
            pragmas: vec![],
            derives: vec![],
        }
    }

    pub(crate) fn with_pragmas<I, T: ToString>(mut self, pragmas: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        self.pragmas = pragmas.into_iter().map(|each| each.to_string()).collect();
        self
    }

    pub(crate) fn with_derives(mut self, derives: &[CustomDerive]) -> Self {
        self.derives = derives.to_vec();
        self
    }

    pub(crate) fn parse(&self) -> Environment {
        let visitor = self.parse_root();
        visitor.into_environment()
    }

    fn parse_root(&self) -> Visitor {
        let mut visitor = Visitor::new(self.crate_root.as_path(), self.start_file.as_path())
            .with_pragmas(&self.pragmas)
            .with_derives(self.derives.clone());

        visitor.enter_mod(&syn::Ident::new(&self.crate_name, Span::call_site()));

        let path = visitor.root_path();
        println!("cargo::rerun-if-changed={}", path.display());

        let mut file = fs::File::open(path)
            .map_err(|e| {
                format!(
                    "{}: {} in {}",
                    e,
                    path.display(),
                    std::env::current_dir().unwrap().display()
                )
            })
            .unwrap();
        let mut content = String::new();
        file.read_to_string(&mut content).unwrap();

        let ast = syn::parse_file(&content).unwrap();

        visitor.visit_file(&ast);

        visitor
    }

    pub(crate) fn export_to(&self, file_path: impl AsRef<std::path::Path>) {
        self.parse().export_to(file_path);
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}
