use proc_macro2::TokenStream;
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

fn expand(attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> Result<TokenStream> {
    let attributes = Attributes::parse(attr)?;
    let item = parse::<Item>(item)?;

    let expanded = match item {
        Item::Fn(item_fn) => AnnotatedFunction::new(item_fn, attributes).expand(),
        Item::Mod(item_mod) => AnnotatedModule::new(item_mod, attributes).expand(),
        _ => syn::Error::new(
            item.span(),
            "Pragmas are not supported for this type of construct",
        )
        .to_compile_error(),
    };

    Ok(expanded)
}
