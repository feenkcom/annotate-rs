use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::ItemMod;
use uuid::Uuid;

use crate::attributes::Attributes;

pub struct AnnotatedModule {
    item_mod: ItemMod,
    attributes: Attributes,
}

impl AnnotatedModule {
    pub fn new(item_mod: ItemMod, attributes: Attributes) -> Self {
        Self {
            item_mod,
            attributes,
        }
    }

    pub fn name(&self) -> &Ident {
        &(self.item_mod.ident)
    }

    pub fn expand(&self) -> TokenStream {
        let item_mod = &self.item_mod;
        let path_to_annotate = self.attributes.path_to_annotate();

        let name = self.name().to_string();
        let expanded_metadata = self.attributes.expand();
        let attrib_fn_ident = Self::generate_function_ident("attr", item_mod);

        quote! {
            #item_mod

            #[unsafe(export_name = concat!("annotate$attr$", module_path!(), "::", #name, "$", file!(), ":", line!()))]
            pub fn #attrib_fn_ident() -> &'static [ #path_to_annotate::Attribute ] {
                #expanded_metadata
                &ATTRIBUTES
            }
        }
    }

    fn generate_function_ident(prefix: &str, item_mod: &ItemMod) -> Ident {
        Ident::new(
            &format!(
                "annotate_mod_{}_{}_{}",
                prefix,
                &item_mod.ident,
                Uuid::new_v4().as_simple()
            ),
            Span::call_site(),
        )
    }
}
