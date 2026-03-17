use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{FnArg, ItemFn};
use uuid::Uuid;
use uuid::fmt::Simple;

use crate::Attributes;

pub struct AnnotatedFunction {
    function: ItemFn,
    attributes: Attributes,
    source_path: String,
}

impl AnnotatedFunction {
    pub fn new(item_fn: ItemFn, attributes: Attributes, source_path: String) -> Self {
        Self {
            function: item_fn,
            attributes,
            source_path,
        }
    }

    pub fn name(&self) -> &Ident {
        &self.function.sig.ident
    }

    pub fn expand(self) -> TokenStream {
        let item_fn = &self.function;
        let path_to_annotate = self.attributes.path_to_annotate();

        let original_ident = self.name();
        let expanded_metadata = self.attributes.expand();

        let input_types: Vec<TokenStream> = Self::extract_input_types(item_fn);

        let original_ident_str = original_ident.to_string();
        let return_type = &item_fn.sig.output;
        let source_path = syn::LitStr::new(self.source_path.as_str(), Span::call_site());

        let wrapper_fn_ident = Self::generate_function_ident("wrapper", item_fn);
        let attrib_fn_ident = Self::generate_function_ident("attr", item_fn);

        let any_return_fn_ident = Self::generate_function_ident("any_return", item_fn);
        let any_return_fn_impl = if input_types.is_empty() {
            quote! {
                fn #any_return_fn_ident() -> #path_to_annotate::__private::AnyReturn {
                    #path_to_annotate::__private::any_return(#original_ident())
                }
            }
        } else {
            quote! {}
        };
        let any_return_pointer = if input_types.is_empty() {
            quote! {Some(#any_return_fn_ident)}
        } else {
            quote! {None}
        };

        quote! {
            #item_fn

            #any_return_fn_impl

            #[unsafe(export_name = concat!("annotate$", module_path!(), "::", #original_ident_str, "$", #source_path, ":", line!()))]
            pub fn #wrapper_fn_ident() -> #path_to_annotate::__private::FunctionPointer {
                #path_to_annotate::__private::function_pointer(
                    &(#original_ident as fn(#(#input_types),*) #return_type) as &dyn std::any::Any,
                    #any_return_pointer
                )
            }

            #[unsafe(export_name = concat!("annotate$attr$", module_path!(), "::", #original_ident_str, "$", #source_path, ":", line!()))]
            pub fn #attrib_fn_ident() -> &'static [ #path_to_annotate::Attribute ] {
                #expanded_metadata
                &ATTRIBUTES
            }
        }
    }

    fn extract_input_types(item_fn: &ItemFn) -> Vec<TokenStream> {
        item_fn
            .sig
            .inputs
            .iter()
            .map(|each| match each {
                FnArg::Receiver(syn::Receiver { ty, .. }) => {
                    quote! { #ty }
                }
                FnArg::Typed(syn::PatType { ty, .. }) => {
                    quote! {
                        #ty
                    }
                }
            })
            .collect()
    }

    fn generate_function_ident(prefix: &str, func: &ItemFn) -> Ident {
        Ident::new(
            &format!(
                "annotate_fn_{}_{}_{}",
                prefix,
                func.sig.ident,
                Simple::from_uuid(Uuid::new_v4())
            ),
            Span::call_site(),
        )
    }
}
