/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

extern crate proc_macro;

use crate::proc_macro::TokenStream;
use quote::quote;

#[proc_macro_derive(Input)]
pub fn input_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_input(&ast)
}

#[proc_macro_derive(Key)]
pub fn key_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_key(&ast)
}

#[proc_macro_derive(DBObj)]
pub fn db_obj_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_db_obj(&ast)
}

#[proc_macro_derive(NamedObj)]
pub fn named_obj_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_named_obj(&ast)
}

fn impl_input(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;

    let gen = quote! {

        impl crate::protocols::Input for #name {
        fn as_any(self: Box<Self>) -> Box<dyn Any> { self }
        fn as_any_ref(&self) -> &dyn Any { self }
        fn as_any_mut(&mut self) -> &mut dyn Any { self }
        fn as_debug(&self) -> &dyn fmt::Debug { self }
    }

    impl<'a> std::convert::TryInto<&'a #name> for &'a Box<dyn crate::protocols::Input> {
        type Error = crate::error::Error;
        fn try_into(self) -> crate::error::Result<&'a Input> {
        Ok(self.as_any_ref().downcast_ref().ok_or_else(
            || crate::error::Error::WrongInput)?)
        }
    }


    impl<'a> std::convert::TryInto<&'a mut #name> for &'a mut Box<dyn crate::protocols::Input> {
        type Error = crate::error::Error;
        fn try_into(self) -> crate::error::Result<&'a mut Input> {
        Ok(self.as_any_mut().downcast_mut().ok_or_else(
            || crate::error::Error::WrongInput)?)
        }
    }

    impl std::convert::TryInto<#name> for Box<dyn crate::protocols::Input> {
        type Error = crate::error::Error;
        fn try_into(self) -> crate::error::Result<Input> {
        Ok(*self.as_any().downcast().map_err(
            |_| crate::error::Error::WrongInput)?)
        }
    }

    };

    gen.into()
}

fn impl_key(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;

    let gen = quote! {
        impl agent_utils::Key for #name {}
        impl std::fmt::Display for #name {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", stringify!(#name), self.0)
        }
    }
    impl From<&str> for #name {
        fn from(val: &str) -> Self {
        Self(String::from(val))
        }
    }
    };

    gen.into()
}

fn impl_db_obj(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;

    let gen = quote! {
        impl agent_utils::DBObj for #name {}
        impl agent_utils::NamedObj for #name {
        const NAME : &'static str = stringify!(#name);
    }
    };

    gen.into()
}

fn impl_named_obj(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;

    let gen = quote! {
        impl utils::NamedObj for #name {
        const NAME : &'static str = stringify!(#name);
    }
    };

    gen.into()
}
