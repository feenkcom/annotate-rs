use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Expr, ExprLit, ExprPath, Lit, MetaNameValue, Path};

const PATH_TO_ANNOTATE_ATTRIBUTE: &str = "path_to_annotate";
const DEFAULT_PATH_TO_ANNOTATE: &str = "annotate";

pub struct Attributes {
    attributes: Punctuated<MetaNameValue, syn::Token![,]>,
    path_to_annotate: TokenStream,
}

impl Attributes {
    pub fn parse(tokens: proc_macro::TokenStream) -> syn::Result<Self> {
        let attributes = Punctuated::parse_terminated.parse(tokens)?;
        let path_to_annotate = Self::detect_path_to_annotate(&attributes);

        Ok(Self {
            attributes,
            path_to_annotate,
        })
    }

    pub fn path_to_annotate(&self) -> &TokenStream {
        &self.path_to_annotate
    }

    fn detect_path_to_annotate(
        attributes: &Punctuated<MetaNameValue, syn::Token![,]>,
    ) -> TokenStream {
        for each in attributes.iter() {
            match each.path.require_ident() {
                Ok(ident) => {
                    if ident.to_string().as_str() == PATH_TO_ANNOTATE_ATTRIBUTE {
                        return match &each.value {
                            Expr::Path(path) => {
                                quote! { #path }
                            }
                            _ => syn::Error::new(
                                each.value.span(),
                                format!(
                                    "Must be a Path (e.g. path::to::annotate) but was {:?}",
                                    &each.path
                                ),
                            )
                            .to_compile_error(),
                        };
                    }
                }
                Err(error) => return error.to_compile_error(),
            }
        }
        let path: Path = syn::parse_str(DEFAULT_PATH_TO_ANNOTATE).unwrap();
        path.to_token_stream()
    }

    pub fn expand(&self) -> TokenStream {
        let path_to_annotate = self.path_to_annotate();
        let attributes = self
            .attributes
            .iter()
            .filter(|each| {
                each.path
                    .get_ident()
                    .map(|name| name.to_string().as_str() != PATH_TO_ANNOTATE_ATTRIBUTE)
                    .unwrap_or(true)
            })
            .map(|each| {
                let name = each
                    .path
                    .require_ident()
                    .map(|ident| {
                        proc_macro2::Literal::string(ident.to_string().as_str()).to_token_stream()
                    })
                    .unwrap_or_else(|error| {
                        syn::Error::new(each.path.span(), error.to_string()).to_compile_error()
                    });
                (name, &each.value)
            })
            .map(|(name, value)| match value {
                Expr::Lit(literal) => {
                    Self::expand_literal_attribute_value(path_to_annotate, name, literal)
                }
                Expr::Path(path) => Self::expand_path_attribute_value(path_to_annotate, name, path),
                _ => syn::Error::new(value.span(), "Unsupported value type").to_compile_error(),
            })
            .collect::<Vec<TokenStream>>();

        let amount = attributes.len();

        quote! {
            const ATTRIBUTES: [#path_to_annotate::Attribute; #amount] = [ #(#attributes),* ];
        }
    }

    fn expand_literal_attribute_value(
        path_to_annotate: &TokenStream,
        name: TokenStream,
        literal: &ExprLit,
    ) -> TokenStream {
        let value = match &literal.lit {
            Lit::Str(value) => {
                quote! { #path_to_annotate::Value::Str(#value) }
            }
            // Lit::ByteStr(_) => {}
            // Lit::CStr(_) => {}
            // Lit::Byte(_) => {}
            // Lit::Char(_) => {}
            Lit::Int(value) => {
                quote! { #path_to_annotate::Value::Int(#value) }
            }
            // Lit::Float(_) => {}
            Lit::Bool(value) => {
                quote! { #path_to_annotate::Value::Bool(#value) }
            }
            _ => syn::Error::new(
                literal.lit.span(),
                format!("Unsupported literal type: {:?}", &literal.lit),
            )
            .to_compile_error(),
        };

        quote! {
            #path_to_annotate::__private::attribute(#name, #value)
        }
    }

    fn expand_path_attribute_value(
        path_to_annotate: &TokenStream,
        name: TokenStream,
        path: &ExprPath,
    ) -> TokenStream {
        quote! {
            #path_to_annotate::__private::attribute(
                #name,
                #path_to_annotate::Value::Type(#path_to_annotate::__private::ty::<#path>()),
            )
        }
    }
}
