extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn;

#[proc_macro_derive(GameType)]
pub fn gametype_derive(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let ast:syn::DeriveInput = syn::parse(input).unwrap();
    let name = &ast.ident;
    let gen = quote! {
        impl GameType for #name {}
    };
    gen.into()
}