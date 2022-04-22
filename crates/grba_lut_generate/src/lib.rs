use proc_macro::TokenStream;
#[macro_use]
mod errors;
mod config;
mod lut_generator;

/// Expand an instruction with const generics to concrete invocations inside a LUT fill function.
///
/// The first argument is the LUT index type `(u8, u16, u32, u64, usize)`, followed by all const generics to template and the
/// bits they are mapped to in the LUT index type.
///
/// This will then generate a function called with the following signature
///
/// ```ignore
/// fn fill_lut_IDENTITY(lookup: LUTINDEXTYPE) -> Option<fn>;
/// ```
///
/// # Example
///
/// The following will create a LUT fill function
///
/// ```ignore
///
/// #[grba_lut_generate::create_lut(u32, SET_FLAGS=4)]
/// fn data_processing<const SET_FLAGS: bool>(cpu: &mut CPU) {
///     // Something to do with SET_FLAGS
/// }
/// ```
///
/// The LUT fill function will then have the following signature:
///
/// ```ignore
/// pub fn fill_lut_data_processing(lookup: u32) -> Option<fn(cpu: &mut CPU)> {
///   // Remainder of LUT...
/// }
/// ```
///
/// The fill function will return `Some` whenever the `lookup` resolves to an invocation.
#[proc_macro_attribute]
pub fn create_lut(args: TokenStream, input: TokenStream) -> TokenStream {
    lut_generator::analyse_and_expand(args.into(), input.into()).into()
}
