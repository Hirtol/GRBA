use std::ops::Deref;

use itertools::Itertools;
use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};
use syn::spanned::Spanned;
use syn::{ItemFn, Pat, Type};

use crate::config::{BitfieldFields, BitfieldType, LutMeta};

/// Analyzes the given token stream for `#[create_lut]` properties and expands code if valid.
pub fn analyse_and_expand(args: TokenStream, input: TokenStream) -> TokenStream {
    match analyse_and_expand_or_error(args, input) {
        Ok(output) => output,
        Err(err) => err.to_compile_error(),
    }
}

/// Analyzes the given token stream for `#[create_lut]` properties and expands code if valid.
///
/// # Errors
///
/// If the given token stream does not yield a valid `#[create_lut]` specifier.
fn analyse_and_expand_or_error(args: TokenStream, input: TokenStream) -> syn::parse::Result<TokenStream> {
    let input = syn::parse::<syn::ItemFn>(input.into())?;
    let args = syn::parse_macro_input::parse::<LutMeta>(args.into())?;

    let gen_info = generate_info(input, args)?;
    let input = &gen_info.fn_input;

    let expanded_lut = expand_lut_fn(&gen_info);

    let output = quote::quote_spanned! {input.span()=>
        #expanded_lut

        #input
    };

    Ok(output)
}

/// Expand the LUT function, for the following example:
///
/// ```rust
///
/// #[grba_lut_generate::create_lut(u32, SET_FLAGS=4)]
/// fn data_processing<const SET_FLAGS: bool>(cpu: &mut CPU) {
///     // Something to do with data processing
/// }
/// ```
///
/// It will look like this:
///
/// ```ignore
/// pub fn fill_lut_data_processing(lookup: u32) -> Option<fn(cpu: &mut CPU)>{
///   if lookup.get_bits(4u8,4u8)==0u8 as u32 {
///     return Some(Self::data_processing:: <{
///       0u8!=0
///     }>);
///   }
///   // Remainder of LUT...
/// }
/// ```
fn expand_lut_fn(gen_info: &LutGeneration) -> TokenStream {
    let index_ident = quote::format_ident!("lookup");
    let lut_index_type = &gen_info.args.lut_index;

    let original_function = &gen_info.fn_input.sig.ident;
    let bitfield_inputs = gen_info
        .bitfield_params
        .iter()
        .map(|i| i.associated_bitfield.b_type.as_inputs())
        .multi_cartesian_product();

    // All if statements + function invocations for the const parameters we have.
    let lut_index_checkers = bitfield_inputs.into_iter().map(|inputs| {
        let bitfield_params = gen_info.bitfield_params.iter().zip(inputs);

        let bool_exprs = bitfield_params.clone().map(|(p, value)| {
            let (begin_bit, end_bin_inclusive) = match &p.associated_bitfield.b_type {
                BitfieldType::Range(range) => (*range.start(), *range.end()),
                BitfieldType::BitIndex(index) => (*index, *index),
            };

            quote::quote! {
                #index_ident.get_bits(#begin_bit, #end_bin_inclusive) == #value as #lut_index_type
            }
        });

        let const_gen_args = bitfield_params.map(|(param, input)| match &param.ty {
            BitfieldParameterType::Bool => {
                quote::quote! {
                    {#input != 0}
                }
            }
            ty => quote::quote! {
                {#input as #ty}
            },
        });

        quote::quote_spanned! {gen_info.fn_input.span()=>
            if #(#bool_exprs)&&* {
                return Some(Self::#original_function::<#(#const_gen_args),*>);
            }
        }
    });

    let fill_lut_ident = quote::format_ident!("fill_lut_{}", gen_info.fn_input.sig.ident);
    let rem_types = gen_info.fn_params.iter().map(|p| &p.original);
    let final_function_type = quote::quote_spanned! {gen_info.fn_input.span()=>
        fn(#(#rem_types),*)
    };

    quote::quote_spanned! {gen_info.fn_input.span()=>
        pub fn #fill_lut_ident(#index_ident: #lut_index_type) -> Option<#final_function_type> {
            #(#lut_index_checkers)*

            None
        }
    }
}

fn generate_info(input: ItemFn, args: LutMeta) -> syn::parse::Result<LutGeneration> {
    let bitfield_params: Vec<_> = args
        .bitfields
        .iter()
        .map(|param| {
            let const_param_found = input.sig.generics.const_params().find(|p| p.ident == param.ident);

            if let Some(const_param) = const_param_found {
                Ok(ConstBitfieldParameter {
                    _ident: const_param.ident.clone(),
                    ty: BitfieldParameterType::try_from(const_param.ty.clone())?,
                    associated_bitfield: param.clone(),
                })
            } else {
                Err(format_err!(
                    param.ident,
                    "Could not find parameter in function signature"
                ))
            }
        })
        .collect::<syn::Result<Vec<_>>>()?;

    let parameters = FunctionParameter::create_from_iter(&input.sig.inputs);

    Ok(LutGeneration {
        fn_input: input,
        args,
        bitfield_params,
        fn_params: parameters,
    })
}

pub struct LutGeneration {
    fn_input: ItemFn,
    fn_params: Vec<FunctionParameter>,
    args: LutMeta,
    bitfield_params: Vec<ConstBitfieldParameter>,
}

struct ConstBitfieldParameter {
    _ident: Ident,
    ty: BitfieldParameterType,
    associated_bitfield: BitfieldFields,
}

enum BitfieldParameterType {
    U8,
    U16,
    U32,
    U64,
    Bool,
}

impl ToTokens for BitfieldParameterType {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            BitfieldParameterType::U8 => tokens.extend(quote! {u8}),
            BitfieldParameterType::U16 => tokens.extend(quote! {u16}),
            BitfieldParameterType::U32 => tokens.extend(quote! {u32}),
            BitfieldParameterType::U64 => tokens.extend(quote! {u64}),
            BitfieldParameterType::Bool => tokens.extend(quote! {bool}),
        }
    }
}

impl TryFrom<syn::Type> for BitfieldParameterType {
    type Error = syn::Error;

    fn try_from(value: Type) -> Result<Self, Self::Error> {
        match value {
            Type::Path(path) => {
                if path.path.is_ident("u8") {
                    Ok(BitfieldParameterType::U8)
                } else if path.path.is_ident("u16") {
                    Ok(BitfieldParameterType::U16)
                } else if path.path.is_ident("u32") {
                    Ok(BitfieldParameterType::U32)
                } else if path.path.is_ident("u64") {
                    Ok(BitfieldParameterType::U64)
                } else if path.path.is_ident("bool") {
                    Ok(BitfieldParameterType::Bool)
                } else {
                    Err(syn::Error::new_spanned(
                        path,
                        "Only `u8`, `u16`, `u32`, `u64` and `bool` are supported as bitfield types",
                    ))
                }
            }
            _ => Err(format_err!(
                value,
                "Expected `u8`, `u16`, `u32`, `u64` or `bool` as bitfield types"
            )),
        }
    }
}

struct FunctionParameter {
    original: syn::FnArg,
    _ident: syn::Ident,
    _ty: syn::Type,
}

impl FunctionParameter {
    pub fn create_from_iter<'a>(iter: impl IntoIterator<Item = &'a syn::FnArg>) -> Vec<Self> {
        iter.into_iter()
            .flat_map(|arg| match &arg {
                syn::FnArg::Typed(typed) => Some((arg, typed)),
                _ => None,
            })
            .flat_map(|(arg, typed)| {
                let iden = match &*typed.pat {
                    Pat::Ident(identity) => identity.ident.clone(),
                    _ => return None,
                };

                Some(FunctionParameter {
                    original: arg.clone(),
                    _ident: iden,
                    _ty: typed.ty.deref().clone(),
                })
            })
            .collect()
    }
}
