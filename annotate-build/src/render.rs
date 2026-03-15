use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use proc_macro2::TokenStream;
use quote::quote;

use crate::{
    AnnotatedFunction, AnnotatedModule, tokenize_modules, tokenize_proto_functions,
    tokenize_proto_modules,
};

pub fn generate(tokens: TokenStream, file_path: impl AsRef<std::path::Path>) {
    let path = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join(file_path.as_ref());

    std::fs::create_dir_all(path.parent().unwrap()).unwrap();

    let mut file = File::create(path.as_path())
        .unwrap_or_else(|_| panic!("Create file {}", path.as_path().display()));

    let source_code = render(tokens);
    file.write_all(source_code.as_bytes()).unwrap();
}

pub fn render(token_stream: TokenStream) -> String {
    // let config = rust_format::Config::new_str()
    //     .edition(rust_format::Edition::Rust2024)
    //     .option("reorder_imports", "false")
    //     .option("reorder_modules", "false")
    //     .option("max_width", "85");
    // let rust_fmt = rust_format::PrettyPlease::from_config(config);
    // rust_fmt.format_tokens(token_stream).unwrap()

    prettyplease::unparse(&syn::parse2(token_stream).unwrap())
}

pub fn tokenize(modules: &[AnnotatedModule], functions: &[AnnotatedFunction]) -> TokenStream {
    let proto_modules = tokenize_proto_modules(modules);
    let proto_functions = tokenize_proto_functions(functions);
    let modules = tokenize_modules(modules);
    let functions = tokenize_functions(functions);

    let proto_environment = quote! {
        const PROTO_ENVIRONMENT: annotate::__private::ProtoEnvironment = annotate::__private::proto_environment(&PROTO_MODULES, &PROTO_FUNCTIONS);
    };

    let environment = quote! {
        pub const ENVIRONMENT: annotate::Environment = annotate::__private::environment(&MODULES, &FUNCTIONS);
    };

    quote! {
        pub (crate) mod __annotate {
            use super::*;

            #proto_modules
            #proto_functions
            #proto_environment
            #modules
            #functions
            #environment
        }
    }
}

pub fn tokenize_functions(functions: &[AnnotatedFunction]) -> TokenStream {
    let amount = functions.len();
    let functions = functions.iter().map(tokenize_function);

    quote! {
        const FUNCTIONS: [annotate::Function; #amount] = [ #(#functions),* ];
    }
}

pub fn tokenize_function(function: &AnnotatedFunction) -> TokenStream {
    let id = function.id();
    quote! {
        annotate::__private::function(&PROTO_ENVIRONMENT, &PROTO_FUNCTIONS[#id])
    }
}
