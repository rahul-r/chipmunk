extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Json)]
pub fn json_macro(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;
    let expanded = quote! {
        impl #name {
            pub fn to_string(&self) -> anyhow::Result<String>
            where
                Self: Serialize,
            {
                serde_json::to_string(&self).map_err(anyhow::Error::msg)
            }

            pub fn from_string<'a>(str: impl Into<&'a str>) -> anyhow::Result<Self>
            where
                Self: Sized,
                for<'b> Self: Deserialize<'b>,
            {
                serde_json::from_str(str.into()).map_err(anyhow::Error::msg)
            }

            pub fn from_value(value: serde_json::Value) -> anyhow::Result<Self>
            where
                Self: Sized,
                for<'c> Self: Deserialize<'c>,
            {
                serde_json::from_value(value).map_err(anyhow::Error::msg)
            }

            pub fn to_value(&self) -> anyhow::Result<serde_json::Value>
            where
                Self: Sized,
                Self: Serialize,
            {
                serde_json::to_value(self).map_err(anyhow::Error::msg)
            }
        }
    };

    TokenStream::from(expanded)
}
