use std::clone;
use std::ops::{Deref, RangeInclusive};

use itertools::Itertools;
use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{parse_macro_input, AttributeArgs, Expr, ExprLit, FnArg, ItemFn, Lit, Pat, Path, RangeLimits, Token, Type};

use crate::config::{BitfieldFields, BitfieldType, LutMeta};
use crate::errors::CombineError;

/// Analyzes the given token stream for `#[bitfield]` properties and expands code if valid.
pub fn analyse_and_expand(args: TokenStream, input: TokenStream) -> TokenStream {
    match analyse_and_expand_or_error(args, input) {
        Ok(output) => output,
        Err(err) => err.to_compile_error(),
    }
}

/// Analyzes the given token stream for `#[bitfield]` properties and expands code if valid.
///
/// # Errors
///
/// If the given token stream does not yield a valid `#[bitfield]` specifier.
fn analyse_and_expand_or_error(args: TokenStream, input: TokenStream) -> syn::parse::Result<TokenStream> {
    let input = syn::parse::<syn::ItemFn>(input.into())?;
    let args = syn::parse_macro_input::parse::<LutMeta>(args.into())?;

    let gen_info = generate_info(input, args)?;
    let input = &gen_info.fn_input;

    let expanded_lut = expand_lut(&gen_info);

    let output = quote::quote_spanned! {input.span()=>
        #expanded_lut

        #input
    };

    Ok(output)
}

fn expand_lut(gen_info: &LutGeneration) -> TokenStream {
    let cartesian_input: Vec<_> = gen_info
        .bitfield_params
        .iter()
        .map(|i| i.associated_bitfield.b_type.as_inputs())
        .multi_cartesian_product()
        .collect();

    let fn_identities = (0..cartesian_input.len())
        .map(|i| quote::format_ident!("{}_gen_{}", gen_info.fn_input.sig.ident, i))
        .collect_vec();
    let function_instances = cartesian_input
        .iter()
        .zip(fn_identities.iter())
        .map(|(bitfield_input, fn_identity)| expand_fn_invocation(gen_info, bitfield_input, fn_identity));

    let fill_lut_ident = quote::format_ident!("fill_lut_{}", gen_info.fn_input.sig.ident);
    let lut_index_type = &gen_info.args.lut_index;
    let rem_types = gen_info.remaining_params.iter().map(|p| &p.original);
    let final_function_type = quote::quote_spanned! {gen_info.fn_input.span()=>
        fn(#(#rem_types),*)
    };
    let lut = expand_fill_lut(gen_info, &fn_identities, &cartesian_input);

    let output = quote::quote_spanned! {gen_info.fn_input.span()=>
        #lut

        #(#function_instances)*
    };

    output
}

fn expand_fill_lut(gen_info: &LutGeneration, identities: &[Ident], bitfield_inputs: &[Vec<u8>]) -> TokenStream {
    let fill_lut_ident = quote::format_ident!("fill_lut_{}", gen_info.fn_input.sig.ident);
    let index_ident = quote::format_ident!("lookup");

    let lut_index_type = &gen_info.args.lut_index;
    let rem_types = gen_info.remaining_params.iter().map(|p| &p.original);
    let final_function_type = quote::quote_spanned! {gen_info.fn_input.span()=>
        fn(#(#rem_types),*)
    };

    let final_if_statements = identities.into_iter().zip(bitfield_inputs).map(|(ident, inputs)| {
        let bitfield_params = gen_info.bitfield_params.iter().zip(inputs).map(|(p, value)| {
            let (begin_bit, end_bin_inclusive) = match &p.associated_bitfield.b_type {
                BitfieldType::Range(range) => (*range.start(), *range.end()),
                BitfieldType::BitIndex(index) => (*index, *index),
            };

            quote::quote! {
                #index_ident.get_bits(#begin_bit, #end_bin_inclusive) == #value as #lut_index_type
            }
        });

        quote::quote_spanned! {ident.span()=>
            if #(#bitfield_params)&&* {
                return Some(Self::#ident);
            }
        }
    });

    let output = quote::quote_spanned! {gen_info.fn_input.span()=>
        pub fn #fill_lut_ident(#index_ident: #lut_index_type) -> Option<#final_function_type> {
            #(#final_if_statements)*

            None
        }
    };

    output
}

fn expand_fn_invocation(gen_info: &LutGeneration, bitfield_input: &[u8], identity: &Ident) -> TokenStream {
    let original_function = &gen_info.fn_input.sig.ident;
    let rem_types = gen_info.remaining_params.iter().map(|p| &p.original);
    let rem_idents = gen_info.remaining_params.iter().map(|p| &p.ident);
    let bitfield_types = gen_info.bitfield_params.iter().map(|p| &p.ty);

    let final_inputs = bitfield_input
        .into_iter()
        .zip(bitfield_types)
        .map(|(input, ty)| match ty {
            BitfieldParameterType::Bool => {
                quote::quote! {
                    #input != 0
                }
            }
            _ => quote::quote! {
                #input as #ty
            },
        });

    let output = quote::quote! {
        fn #identity(#(#rem_types),*) {
            Self::#original_function(#(#rem_idents),*, #(#final_inputs),*)
        }
    };

    output
}

fn generate_info(input: ItemFn, args: LutMeta) -> syn::parse::Result<LutGeneration> {
    let mut parameters = FunctionParameter::create_from_iter(&input.sig.inputs);

    let bitfield_params: Vec<_> = args
        .bitfields
        .iter()
        .map(|param| {
            let fn_param_idx = parameters
                .iter()
                .enumerate()
                .find(|(_, p)| p.ident == param.ident)
                .map(|(i, _)| i);

            if let Some(i) = fn_param_idx {
                // Remove it from the remaining parameters.
                let fn_param = parameters.remove(i);

                Ok(BitfieldParameter {
                    ident: fn_param.ident.clone(),
                    ty: BitfieldParameterType::try_from(fn_param.ty.clone())?,
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

    Ok(LutGeneration {
        fn_input: input,
        args,
        bitfield_params,
        remaining_params: parameters,
    })
}

pub struct LutGeneration {
    fn_input: ItemFn,
    args: LutMeta,
    bitfield_params: Vec<BitfieldParameter>,
    remaining_params: Vec<FunctionParameter>,
}

struct BitfieldParameter {
    ident: syn::Ident,
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

#[derive(Clone)]
struct FunctionParameter {
    original: syn::FnArg,
    ident: syn::Ident,
    ty: syn::Type,
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
                    ident: iden,
                    ty: typed.ty.deref().clone(),
                })
            })
            .collect()
    }
}
