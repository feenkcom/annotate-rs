use std::collections::{HashMap, LinkedList};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use mustache::Template;
use proc_macro2::Span;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::visit::{Visit, visit_item_fn, visit_item_foreign_mod, visit_item_mod, visit_item_struct};
use syn::{Attribute, Ident, ItemFn, ItemForeignMod, ItemMod, ItemStruct, Meta, PathSegment};
use uuid::Uuid;

use crate::environment::Environment;
use crate::{
    AnnotatedFunction, AnnotatedFunctionData, AnnotatedModule, FunctionPath, ModuleDeriveSpec,
};

#[derive(Debug)]
pub struct Visitor {
    custom_pragmas: Vec<String>,
    custom_derives: Vec<CustomDerive>,
    custom_module_derives: HashMap<String, Vec<CustomModuleDerive>>,
    current_mod: syn::Path,
    current_path: PathBuf,
    // when set, overrides the value from span
    current_line: Option<usize>,
    root_path: PathBuf,
    crate_root: PathBuf,
    // keeps track of the nested modules
    annotated_modules_stack: LinkedList<AnnotatedModule>,
    annotated_modules: Vec<AnnotatedModule>,
    annotated_functions: Vec<AnnotatedFunction>,
}

#[derive(Debug, Clone)]
pub(crate) struct CustomDerive {
    name: String,
    template: Rc<Template>,
}

impl CustomDerive {
    /// Create a custom derive mapping from a derive name to a mustache template.
    pub(crate) fn new(name: impl Into<String>, template: impl AsRef<str>) -> Self {
        let template = mustache::compile_str(template.as_ref()).unwrap();
        Self {
            name: name.into(),
            template: Rc::new(template),
        }
    }

    pub fn render(&self, struct_name: String) -> String {
        let mut data = HashMap::new();
        data.insert("struct_ident", struct_name.to_lowercase());

        self.template.render_to_string(&data).unwrap()
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct CustomModuleDerive {
    name: Option<String>,
    function_names: Vec<String>,
    modules: Vec<CustomModuleDerive>,
}

impl CustomModuleDerive {
    pub fn new(name: String) -> Self {
        Self {
            name: Some(name),
            function_names: vec![],
            modules: vec![],
        }
    }

    pub fn add_function(&mut self, function_name: String) {
        self.function_names.push(function_name);
    }

    pub fn add_module(&mut self, module_name: String) -> usize {
        let len = self.modules.len();
        self.modules.push(Self::new(module_name));
        len
    }

    pub fn derive_at_mut(&mut self, path: &[usize]) -> &mut Self {
        if path.is_empty() {
            return self;
        }

        let first_index = *path.first().expect("Path is empty");
        let amount_of_modules = self.modules.len();
        let module = self.modules.get_mut(first_index).unwrap_or_else(|| {
            panic!(
                "Could not find module at index {}. There are only {} modules",
                first_index, amount_of_modules
            );
        });

        module.derive_at_mut(&path[1..])
    }

    pub(crate) fn from_spec(spec: &ModuleDeriveSpec) -> Self {
        let mut derive = match spec.name {
            Some(name) => Self::new(name.to_string()),
            None => Self::default(),
        };

        for function in spec.functions {
            derive.add_function((*function).to_string());
        }

        for module in spec.modules {
            derive.modules.push(Self::from_spec(module));
        }

        derive
    }

    #[cfg(test)]
    pub(crate) fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    #[cfg(test)]
    pub(crate) fn function_names(&self) -> &[String] {
        self.function_names.as_slice()
    }

    #[cfg(test)]
    pub(crate) fn modules(&self) -> &[CustomModuleDerive] {
        self.modules.as_slice()
    }
}

impl Visitor {
    pub fn new(crate_root: impl Into<PathBuf>, path: impl Into<PathBuf>) -> Self {
        let root_path = path.into();
        Self {
            custom_pragmas: vec![],
            custom_derives: vec![],
            custom_module_derives: Default::default(),
            current_mod: syn::Path {
                leading_colon: None,
                segments: Default::default(),
            },
            current_line: None,
            root_path: root_path.clone(),
            crate_root: crate_root.into(),
            current_path: root_path,
            annotated_modules: vec![],
            annotated_modules_stack: Default::default(),
            annotated_functions: vec![],
        }
    }

    pub fn with_pragmas<I, T: ToString>(mut self, pragmas: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        self.custom_pragmas
            .extend(pragmas.into_iter().map(|each| each.to_string()));
        self
    }

    pub fn with_derives(mut self, derives: Vec<CustomDerive>) -> Self {
        self.custom_derives.extend(derives);
        self
    }

    pub fn with_module_derives(
        mut self,
        module_derives: HashMap<String, Vec<CustomModuleDerive>>,
    ) -> Self {
        for (key, value) in module_derives {
            self.custom_module_derives
                .entry(key)
                .and_modify(|existing| {
                    existing.extend(value.clone());
                })
                .or_insert_with(|| value);
        }
        self
    }

    pub fn root_path(&self) -> &Path {
        self.root_path.as_path()
    }

    pub fn into_environment(self) -> Environment {
        Environment {
            modules: self.annotated_modules,
            functions: self.annotated_functions,
        }
    }

    fn current_dir(&self) -> &Path {
        if self.current_path.is_dir() {
            self.current_path.as_path()
        } else {
            self.current_path.parent().unwrap()
        }
    }

    // Relative to the workspace
    fn current_file(&self) -> &Path {
        self.current_path.as_path()
    }

    fn visit_external_mod(&mut self, i: &ItemMod) {
        let file_or_dir_name = i.ident.to_string();
        let mut file_or_dir_path = self.current_dir().join(file_or_dir_name.as_str());
        if file_or_dir_path.is_dir() {
            let previous_path = self.current_path.clone();
            self.current_path = file_or_dir_path;
            self.visit_directory_mod(&self.current_path.clone());
            self.current_path = previous_path;
            return;
        }
        file_or_dir_path.set_extension("rs");

        if file_or_dir_path.is_file() {
            let previous_path = self.current_path.clone();
            self.current_path = file_or_dir_path;
            self.visit_file_mod(&self.current_path.clone());
            self.current_path = previous_path;
            return;
        }
        panic!("{} doesn't exist", file_or_dir_path.display())
    }

    fn visit_directory_mod(&mut self, _dir: &Path) {
        //todo!("implement directory mod");
    }

    fn visit_file_mod(&mut self, file: &Path) {
        println!("cargo::rerun-if-changed={}", file.display());

        let mut file = fs::File::open(file).unwrap();
        let mut content = String::new();
        file.read_to_string(&mut content).unwrap();

        let ast = syn::parse_file(&content).unwrap();

        self.visit_file(&ast);
    }

    fn visit_custom_module_derive(&mut self, module: &AnnotatedModule, derive: CustomModuleDerive) {
        if let Some(ref module_name) = derive.name {
            self.enter_mod(&Ident::new(module_name.as_str(), Span::call_site()));
        }

        for each_function in derive.function_names.as_slice() {
            self.add_annotated_function(
                Ident::new(&each_function, Span::call_site()),
                module.line(),
            );
        }

        for each_module in derive.modules.as_slice() {
            self.visit_custom_module_derive(module, each_module.clone());
        }

        if derive.name.is_some() {
            self.pop_mod();
        }
    }

    pub fn enter_mod(&mut self, module: &Ident) {
        self.current_mod.segments.push(PathSegment {
            ident: module.clone(),
            arguments: Default::default(),
        });
    }

    fn pop_mod(&mut self) {
        self.current_mod.segments.pop();
    }

    fn is_annotation(&self, attr: &Attribute) -> bool {
        self.get_annotation_name(attr).is_some()
    }

    fn get_annotation_name(&self, attr: &Attribute) -> Option<String> {
        if let Some(segment) = attr.path().segments.last() {
            let pragma = segment.ident.to_string();
            if pragma.as_str() == "pragma" {
                return Some(pragma);
            }
            if self.custom_pragmas.contains(&pragma) {
                return Some(pragma);
            }
        }
        None
    }

    fn get_custom_derives(&self, attr: &Attribute) -> Vec<CustomDerive> {
        if let Meta::List(meta_list) = &attr.meta
            && let Some(segment) = meta_list.path.segments.last()
        {
            let ident = segment.ident.to_string();
            if ident.as_str() == "derive" {
                let tokens = &meta_list.tokens;
                let derives: Punctuated<syn::Path, syn::Token![,]> =
                    Punctuated::parse_terminated.parse2(tokens.clone()).unwrap();

                return self
                    .custom_derives
                    .iter()
                    .filter(|each_custom_derive| {
                        derives.iter().any(|each_derive| {
                            let each_derive_name =
                                each_derive.segments.last().unwrap().ident.to_string();

                            each_derive_name.as_str() == each_custom_derive.name.as_str()
                        })
                    })
                    .cloned()
                    .collect();
            }
        }

        vec![]
    }

    fn add_annotated_function(&mut self, function_name: Ident, line: usize) {
        let mut function_path: FunctionPath = (&self.current_mod).into();
        function_path.push(function_name.clone());

        let annotated_function = AnnotatedFunction(Rc::new(AnnotatedFunctionData {
            id: self.annotated_functions.len(),
            function_name,
            function_path,
            uuid: Uuid::new_v4(),
            line,
            file: self.crate_root.join(self.current_file()),
            annotated_module: self.annotated_modules_stack.front().map(|m| m.as_weak()),
        }));

        if let Some(parent_module) = self.annotated_modules_stack.front() {
            parent_module.add_function(annotated_function.clone());
        }

        self.annotated_functions.push(annotated_function)
    }

    fn add_annotated_module(&mut self, module_name: Ident, line: usize) -> AnnotatedModule {
        let annotated_module = AnnotatedModule::new(
            self.annotated_modules.len(),
            module_name,
            self.current_mod.clone().into(),
            self.annotated_modules_stack.front(),
            line,
            self.crate_root.join(self.current_file()),
        );

        if let Some(parent_module) = self.annotated_modules_stack.front() {
            parent_module.add_module(annotated_module.clone());
        }

        self.annotated_modules_stack
            .push_front(annotated_module.clone());
        self.annotated_modules.push(annotated_module.clone());
        annotated_module
    }
}

impl<'ast> Visit<'ast> for Visitor {
    fn visit_item_fn(&mut self, i: &'ast ItemFn) {
        if let Some(attribute) = i.attrs.iter().find(|each| self.is_annotation(each)) {
            let function_name = i.sig.ident.clone();
            let line = self
                .current_line
                .unwrap_or_else(|| attribute.span().start().line);
            self.add_annotated_function(function_name, line);
        }
        visit_item_fn(self, i)
    }

    fn visit_item_foreign_mod(&mut self, i: &'ast ItemForeignMod) {
        visit_item_foreign_mod(self, i)
    }

    fn visit_item_mod(&mut self, i: &'ast ItemMod) {
        let mut entered_annotated_module = false;

        self.enter_mod(&i.ident);

        if i.content.is_none() {
            self.visit_external_mod(i);
        } else {
            if let Some(attribute) = i.attrs.iter().find(|each| self.is_annotation(each)) {
                let annotated_module = self.add_annotated_module(
                    i.ident.clone(),
                    self.current_line
                        .unwrap_or_else(|| attribute.span().start().line),
                );

                entered_annotated_module = true;

                if let Some(ref annotation_name) = self.get_annotation_name(attribute) {
                    if let Some(module_derives) =
                        self.custom_module_derives.get(annotation_name).cloned()
                    {
                        for each in module_derives {
                            self.visit_custom_module_derive(&annotated_module, each);
                        }
                    }
                }
            }
        }

        visit_item_mod(self, i);
        self.pop_mod();

        if entered_annotated_module {
            self.annotated_modules_stack.pop_front();
        }
    }

    fn visit_item_struct(&mut self, i: &'ast ItemStruct) {
        for attribute in i.attrs.iter() {
            for derive in self.get_custom_derives(attribute) {
                let previous_line = self.current_line.take();
                self.current_line = Some(attribute.span().start().line);

                let derive_source = derive.render(i.ident.to_string());
                let parsed_mod: ItemMod = syn::parse_str(&derive_source).unwrap();
                self.visit_item_mod(&parsed_mod);
                self.current_line = previous_line;
            }
        }
        visit_item_struct(self, i)
    }
}
