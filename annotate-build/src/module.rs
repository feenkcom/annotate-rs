use std::cell::RefCell;
use std::fmt::{Display, Formatter};
use std::rc::{Rc, Weak};

use proc_macro2::{Ident, Literal, Span, TokenStream};
use quote::{ToTokens, quote};
use uuid::Uuid;

use crate::{AnnotatedFunction, TypedPath};

pub type ModulePath = TypedPath<AnnotatedModule>;

#[derive(Debug, Clone)]
pub struct AnnotatedModule(Rc<AnnotatedModuleData>);

#[derive(Debug, Clone)]
pub struct WeakAnnotatedModule(Weak<AnnotatedModuleData>);

#[derive(Debug)]
struct AnnotatedModuleData {
    pub id: usize,
    pub name: Ident,
    pub path: ModulePath,
    pub parent: Option<WeakAnnotatedModule>,
    pub modules: RefCell<Vec<AnnotatedModule>>,
    pub functions: RefCell<Vec<AnnotatedFunction>>,
    pub uuid: Uuid,
    pub line: usize,
    pub source_path: String,
}

impl AnnotatedModule {
    pub fn new(
        id: usize,
        name: Ident,
        path: ModulePath,
        parent: Option<&AnnotatedModule>,
        line: usize,
        source_path: String,
    ) -> Self {
        Self(Rc::new(AnnotatedModuleData {
            id,
            name,
            path,
            parent: parent.map(|module| module.as_weak()),
            modules: RefCell::new(vec![]),
            functions: RefCell::new(vec![]),
            uuid: Uuid::new_v4(),
            line,
            source_path,
        }))
    }

    pub fn id(&self) -> usize {
        self.0.id
    }

    pub fn name_ident(&self) -> &Ident {
        &self.0.name
    }
    pub fn name_literal(&self) -> Literal {
        Literal::string(self.name_ident().to_string().as_str())
    }

    pub fn path(&self) -> &ModulePath {
        &self.0.path
    }

    pub fn add_module(&self, module: AnnotatedModule) {
        self.0.modules.borrow_mut().push(module);
    }

    pub fn add_function(&self, function: AnnotatedFunction) {
        self.0.functions.borrow_mut().push(function);
    }

    pub(crate) fn as_weak(&self) -> WeakAnnotatedModule {
        WeakAnnotatedModule(Rc::downgrade(&self.0))
    }

    pub fn parent_id(&self) -> Option<usize> {
        self.0
            .parent
            .as_ref()
            .and_then(|module| module.upgrade())
            .map(|module| module.id())
    }

    fn uuid(&self) -> &Uuid {
        &self.0.uuid
    }

    pub(crate) fn line(&self) -> usize {
        self.0.line
    }

    pub fn generate_attributes_function_link_name(&self) -> String {
        format!(
            "annotate$attr${}${}:{}",
            self.path(),
            self.0.source_path,
            self.line()
        )
    }

    pub fn generate_attributes_function_identifier(&self) -> Ident {
        let name = format!(
            "annotate_mod_attrib_{}_{}",
            self.path().to_string().replace("::", "_"),
            self.uuid().as_simple()
        );
        Ident::new(&name, Span::call_site())
    }
}

impl WeakAnnotatedModule {
    pub fn upgrade(&self) -> Option<AnnotatedModule> {
        self.0.upgrade().map(AnnotatedModule)
    }
}

impl Display for AnnotatedModule {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "mod {} {{ ... }}", &self.0.path)
    }
}

pub struct TokenizeProtoModule<'a>(&'a AnnotatedModule);

impl<'a> From<&'a AnnotatedModule> for TokenizeProtoModule<'a> {
    fn from(value: &'a AnnotatedModule) -> Self {
        Self(value)
    }
}

impl ToTokens for TokenizeProtoModule<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let module = &self.0;
        let name = module.name_literal();
        let path = module.path().expand_as_const_path();
        let attributes = module.generate_attributes_function_identifier();
        let submodules = module
            .0
            .modules
            .borrow()
            .iter()
            .map(|module| module.id())
            .collect::<Vec<usize>>();
        let functions = module
            .0
            .functions
            .borrow()
            .iter()
            .map(|functions| functions.id())
            .collect::<Vec<usize>>();

        let parent_module = module
            .parent_id()
            .map(|id| quote! { Some(#id)})
            .unwrap_or_else(|| quote! {None});
        tokens.extend(quote! {
            annotate::__private::proto_module(
                #name,
                #path,
                &[#(#submodules),*],
                &[#(#functions),*],
                #parent_module,
                #attributes,
            )
        })
    }
}

pub struct TokenizeModule<'a>(&'a AnnotatedModule);

impl<'a> From<&'a AnnotatedModule> for TokenizeModule<'a> {
    fn from(value: &'a AnnotatedModule) -> Self {
        Self(value)
    }
}

impl ToTokens for TokenizeModule<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let id = self.0.id();

        tokens.extend(quote! {
            annotate::__private::module(&PROTO_ENVIRONMENT, &PROTO_MODULES[#id])
        })
    }
}

fn tokenize_extern_functions(module: &AnnotatedModule) -> TokenStream {
    let attrib_ident = module.generate_attributes_function_identifier();
    let attrib_link_name = module.generate_attributes_function_link_name();

    quote! {
        #[link_name = #attrib_link_name]
        fn #attrib_ident() -> &'static [annotate::Attribute];
    }
}

pub fn tokenize_proto_modules(modules: &[AnnotatedModule]) -> TokenStream {
    let amount = modules.len();
    let tokenized_modules: Vec<TokenizeProtoModule> =
        modules.iter().map(|each| each.into()).collect();
    let extern_functions: Vec<TokenStream> =
        modules.iter().map(tokenize_extern_functions).collect();

    quote! {
        const PROTO_MODULES: [annotate::__private::ProtoModule; #amount] = [ #(#tokenized_modules),* ];

        unsafe extern "Rust" {
            #(#extern_functions)*
        }
    }
}

pub fn tokenize_modules(modules: &[AnnotatedModule]) -> TokenStream {
    let amount = modules.len();
    let modules: Vec<TokenizeModule> = modules.iter().map(|each| each.into()).collect();

    quote! {
        const MODULES: [annotate::Module; #amount] = [ #(#modules),* ];
    }
}
