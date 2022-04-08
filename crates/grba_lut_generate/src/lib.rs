use proc_macro::TokenStream;
#[macro_use]
mod errors;
mod config;
mod lut_generator;

#[proc_macro_attribute]
pub fn create_lut(args: TokenStream, input: TokenStream) -> TokenStream {
    lut_generator::analyse_and_expand(args.into(), input.into()).into()
}
