use std::fmt::{Debug, Display, Formatter};
use std::rc::Rc;

use proc_macro2::{Ident, Literal, Span, TokenStream};
use quote::{ToTokens, quote};
use uuid::Uuid;

use crate::{TypedPath, WeakAnnotatedModule};

pub type FunctionPath = TypedPath<AnnotatedFunction>;

#[derive(Debug, Clone)]
pub struct AnnotatedFunction(pub Rc<AnnotatedFunctionData>);

#[derive(Debug)]
pub struct AnnotatedFunctionData {
    pub id: usize,
    pub function_name: Ident,
    pub function_path: FunctionPath,
    pub uuid: Uuid,
    pub line: usize,
    pub source_path: String,
    pub annotated_module: Option<WeakAnnotatedModule>,
}

impl AnnotatedFunction {
    pub fn name(&self) -> &Ident {
        &self.0.function_name
    }

    pub fn id(&self) -> usize {
        self.0.id
    }

    pub fn module_id(&self) -> Option<usize> {
        self.0
            .annotated_module
            .as_ref()
            .and_then(|module| module.upgrade())
            .map(|module| module.id())
    }

    pub fn link_name(&self) -> String {
        format!(
            "annotate${}${}:{}",
            self.0.function_path, self.0.source_path, self.0.line
        )
    }

    pub fn attrib_link_name(&self) -> String {
        format!(
            "annotate$attr${}${}:{}",
            self.0.function_path, self.0.source_path, self.0.line
        )
    }

    pub fn generated_function_name(&self) -> Ident {
        let name = format!(
            "annotate_fn_{}_{}",
            self.0.function_path.to_string().replace("::", "_"),
            self.0.uuid.as_simple()
        );
        Ident::new(&name, Span::call_site())
    }

    pub fn generate_attributes_function_identifier(&self) -> Ident {
        let name = format!(
            "annotate_fn_attrib_{}_{}",
            self.0.function_path.to_string().replace("::", "_"),
            self.0.uuid.as_simple()
        );
        Ident::new(&name, Span::call_site())
    }
}

impl Display for AnnotatedFunction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "fn {} {{ ... }}", &self.0.function_path)
    }
}

pub struct TokenizeProtoFunction<'a>(&'a AnnotatedFunction);

impl<'a> From<&'a AnnotatedFunction> for TokenizeProtoFunction<'a> {
    fn from(value: &'a AnnotatedFunction) -> Self {
        Self(value)
    }
}

impl ToTokens for TokenizeProtoFunction<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let function = self.0;
        let name = Literal::string(function.name().to_string().as_str());
        let path = function.0.function_path.expand_as_const_path();
        let attributes = function.generate_attributes_function_identifier();
        let extern_name = function.generated_function_name();
        let module = function
            .module_id()
            .map(|id| quote! { Some(#id)})
            .unwrap_or_else(|| quote! {None});

        let expanded_tokens = quote! {
            annotate::__private::proto_function(#name, #path, #module, #attributes, #extern_name)
        };
        tokens.extend(expanded_tokens);
    }
}

fn tokenize_extern_functions(function: &AnnotatedFunction) -> TokenStream {
    let ident = function.generated_function_name();
    let link_name = function.link_name();

    let attrib_ident = function.generate_attributes_function_identifier();
    let attrib_link_name = function.attrib_link_name();

    quote! {
        #[link_name = #link_name]
        fn #ident() -> annotate::__private::FunctionPointer;
        #[link_name = #attrib_link_name]
        fn #attrib_ident() -> &'static [annotate::Attribute];
    }
}

pub fn tokenize_proto_functions(functions: &[AnnotatedFunction]) -> TokenStream {
    let proto_functions: Vec<TokenizeProtoFunction> =
        functions.iter().map(TokenizeProtoFunction::from).collect();
    let extern_functions: Vec<TokenStream> =
        functions.iter().map(tokenize_extern_functions).collect();
    let amount = proto_functions.len();

    quote! {
        pub const PROTO_FUNCTIONS: [annotate::__private::ProtoFunction; #amount] = [ #(#proto_functions),* ];

        unsafe extern "Rust" {
            #(#extern_functions)*
        }
    }
}
