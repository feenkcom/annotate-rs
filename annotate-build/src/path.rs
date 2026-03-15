use proc_macro2::{Ident, Literal, TokenStream};
use quote::{ToTokens, quote};
use std::fmt::{Debug, Display, Formatter};
use std::marker::PhantomData;

#[repr(transparent)]
pub struct TypedPath<T>(Vec<Ident>, PhantomData<T>);

impl<T> TypedPath<T> {
    pub fn new<I>(idents: I) -> Self
    where
        I: IntoIterator<Item = Ident>,
    {
        Self(idents.into_iter().collect(), Default::default())
    }

    pub fn push(&mut self, ident: Ident) {
        self.0.push(ident);
    }

    pub fn expand_as_const_path(&self) -> TokenStream {
        let path: Vec<Literal> = self
            .0
            .iter()
            .map(|each| Literal::string(each.to_string().as_str()))
            .collect();

        quote! { annotate::__private::path(&[ #(#path),* ]) }
    }
}

impl<T> Debug for TypedPath<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", &self.0)
    }
}

impl<T> Clone for TypedPath<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1)
    }
}

impl<T> Display for TypedPath<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut iter = self.0.iter();
        if let Some(arg) = iter.next() {
            write!(f, "{}", arg)?;
        }
        for arg in iter {
            write!(f, "::{}", arg)?;
        }
        Ok(())
    }
}

impl<T> From<syn::Path> for TypedPath<T> {
    fn from(value: syn::Path) -> Self {
        Self::new(value.segments.into_iter().map(|each| each.ident))
    }
}

impl<T> From<&syn::Path> for TypedPath<T> {
    fn from(value: &syn::Path) -> Self {
        Self::new(value.segments.iter().map(|each| each.ident.clone()))
    }
}

impl<A, B> From<&TypedPath<A>> for TypedPath<B> {
    fn from(value: &TypedPath<A>) -> Self {
        Self::new(value.0.clone())
    }
}

impl<T> ToTokens for TypedPath<T> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let idents = self.0.as_slice();
        tokens.extend(quote! { #(#idents)::* })
    }
}
