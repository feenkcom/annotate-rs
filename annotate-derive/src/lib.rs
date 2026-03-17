use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;
use syn::{Item, Result, parse};

pub(crate) use attributes::*;

use crate::function::AnnotatedFunction;
use crate::module::AnnotatedModule;

mod attributes;
mod function;
mod module;

#[proc_macro_attribute]
pub fn pragma(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let expanded = expand(attr, item).unwrap_or_else(|error| error.to_compile_error());
    expanded.into()
}

#[proc_macro]
pub fn environment(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let expanded = expand_environment(input).unwrap_or_else(|error| error.to_compile_error());
    expanded.into()
}

fn expand(attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> Result<TokenStream> {
    let source_path = source_path_of(item.clone());
    let attributes = Attributes::parse(attr)?;
    let item = parse::<Item>(item)?;

    let expanded = match item {
        Item::Fn(item_fn) => AnnotatedFunction::new(item_fn, attributes, source_path).expand(),
        Item::Mod(item_mod) => AnnotatedModule::new(item_mod, attributes, source_path).expand(),
        _ => syn::Error::new(
            item.span(),
            "Pragmas are not supported for this type of construct",
        )
        .to_compile_error(),
    };

    Ok(expanded)
}

fn expand_environment(input: proc_macro::TokenStream) -> Result<TokenStream> {
    let annotate_path = if input.is_empty() {
        None
    } else {
        Some(parse::<syn::Path>(input)?)
    };
    let generated_path = environment_source_path(proc_macro::Span::call_site());
    let generated_path = syn::LitStr::new(generated_path.as_str(), proc_macro2::Span::call_site());

    let expanded = if let Some(annotate_path) = annotate_path {
        quote! {
            mod __annotate {
                use #annotate_path;
                include!(concat!(env!("OUT_DIR"), "/annotate/", #generated_path));
                pub const fn environment() -> &'static #annotate_path::Environment {
                    &__annotate::ENVIRONMENT
                }
            }
        }
    } else {
        quote! {
            #[macro_use]
            extern crate annotate;
            extern crate alloc;

            include!(concat!(env!("OUT_DIR"), "/annotate/", #generated_path));
            pub const fn environment() -> &'static annotate::Environment {
                &__annotate::ENVIRONMENT
            }
        }
    };

    Ok(expanded)
}

fn environment_source_path(span: proc_macro::Span) -> String {
    let source_path = std::path::PathBuf::from(span.file());
    let manifest_root = std::path::PathBuf::from(
        std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .file_name()
            .unwrap(),
    );

    if source_path.is_absolute()
        && let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR")
    {
        let manifest_dir = std::path::PathBuf::from(manifest_dir);
        if let Ok(relative_path) = source_path.strip_prefix(&manifest_dir) {
            return manifest_root
                .join(relative_path)
                .to_string_lossy()
                .replace('\\', "/");
        }
    }

    if source_path
        .components()
        .next()
        .map(|component| component.as_os_str() == manifest_root.as_os_str())
        .unwrap_or(false)
    {
        return source_path.to_string_lossy().replace('\\', "/");
    }

    manifest_root.join(source_path).to_string_lossy().replace('\\', "/")
}

fn source_path_of(stream: proc_macro::TokenStream) -> String {
    let span = stream
        .into_iter()
        .next()
        .map(|token| token.span())
        .unwrap_or_else(proc_macro::Span::call_site);
    environment_source_path(span)
}
