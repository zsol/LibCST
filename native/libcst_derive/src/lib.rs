mod inflate;
use inflate::impl_inflate;
mod parenthesized_node;
use parenthesized_node::impl_parenthesized_node;
mod codegen;
use codegen::impl_codegen;
mod into_py;
use into_py::impl_into_py;

use proc_macro::TokenStream;

#[proc_macro_derive(Inflate)]
pub fn inflate_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_inflate(&ast)
}

#[proc_macro_derive(ParenthesizedNode)]
pub fn parenthesized_node_derive(input: TokenStream) -> TokenStream {
    impl_parenthesized_node(&syn::parse(input).unwrap())
}

#[proc_macro_derive(Codegen)]
pub fn parenthesized_node_codegen(input: TokenStream) -> TokenStream {
    impl_codegen(&syn::parse(input).unwrap())
}

#[proc_macro_derive(IntoPy, attributes(skip_py))]
pub fn into_py(input: TokenStream) -> TokenStream {
    impl_into_py(&syn::parse(input).unwrap())
}
