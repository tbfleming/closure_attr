#![doc = include_str!("../README.md")]

use proc_macro::TokenStream;

#[proc_macro_attribute]
/// See [closure_attr](https://crates.io/crates/closure_attr).
pub fn with_closure(attr: TokenStream, item: TokenStream) -> TokenStream {
    closure_attr_core::with_closure(attr.into(), item.into()).into()
}
