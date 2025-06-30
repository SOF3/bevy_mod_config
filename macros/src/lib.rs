use proc_macro::TokenStream;

mod config;

#[proc_macro_derive(Config, attributes(config))]
pub fn config_derive(input: TokenStream) -> TokenStream {
    config::derive(input.into()).unwrap_or_else(syn::Error::into_compile_error).into()
}
