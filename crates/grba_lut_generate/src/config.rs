use crate::errors::CombineError;
use quote::{quote, ToTokens};
use std::ops::RangeInclusive;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Expr, Lit, Path, RangeLimits, Token};

pub struct LutMeta {
    pub lut_index: ReprKind,
    pub bitfields: Vec<BitfieldFields>,
}

impl syn::parse::Parse for LutMeta {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let index_type: ReprKind = input.parse()?;
        let lookahead = input.lookahead1();

        if lookahead.peek(Token![,]) {
            let _: Token![,] = input.parse()?;
            let list: Punctuated<BitfieldFields, Token![,]> = input.parse_terminated(BitfieldFields::parse)?;

            Ok(Self {
                lut_index: index_type,
                bitfields: list.into_iter().collect(),
            })
        } else {
            Ok(Self {
                lut_index: index_type,
                bitfields: Vec::new(),
            })
        }
    }
}

#[derive(Clone)]
pub struct BitfieldFields {
    /// The Identity
    pub ident: syn::Ident,
    pub eq_token: Token![=],
    pub b_type: BitfieldType,
}

impl Parse for BitfieldFields {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(BitfieldFields {
            ident: input.parse()?,
            eq_token: input.parse()?,
            b_type: input.parse()?,
        })
    }
}

#[derive(Debug, Clone)]
pub enum BitfieldType {
    Range(RangeInclusive<u8>),
    BitIndex(u8),
}

impl BitfieldType {
    /// Convert self to the expected possible inputs for a bitfield
    ///
    /// For example, if a function were to take a `is_load: bool` flag then that would be [BitfieldType::BitIndex], and
    /// have two possible inputs `[0, 1]`.
    ///
    /// For [BitfieldType::Range] we instead want all possible values that that bitrange could represent.
    /// E.g, if the bitrange is `2..=3` then we want `[0, 1, 2, 3]` as possible inputs.
    pub fn as_inputs(&self) -> Vec<u8> {
        match self {
            BitfieldType::Range(range) => {
                let len = range.len() as u32;

                (0..2u8.pow(len)).collect()
            }
            BitfieldType::BitIndex(_) => vec![0, 1],
        }
    }
}

impl Parse for BitfieldType {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let general_expr: syn::Expr = input.parse()?;

        match general_expr {
            Expr::Range(range) => match range.limits {
                RangeLimits::HalfOpen(_) => {
                    let start = range.from.and_then(|expr| parse_int(*expr).ok()).unwrap_or_default();
                    let end = range.to.and_then(|expr| parse_int(*expr).ok()).unwrap_or(u8::MAX);

                    Ok(BitfieldType::Range(start..=end.saturating_sub(1)))
                }
                RangeLimits::Closed(_) => {
                    let start = range.from.and_then(|expr| parse_int(*expr).ok()).unwrap_or_default();
                    let end = range.to.and_then(|expr| parse_int(*expr).ok()).unwrap_or(u8::MAX);
                    Ok(BitfieldType::Range(start..=end))
                }
            },
            Expr::Lit(lit) => Ok(BitfieldType::BitIndex(parse_int(Expr::Lit(lit))?)),

            undefined => Err(format_err!(undefined, "Expected a range or an integer literal")),
        }
    }
}

fn parse_int(input: Expr) -> syn::Result<u8> {
    match input {
        Expr::Lit(lit) => match lit.lit {
            Lit::Int(lit) => Ok(lit.base10_parse()?),
            _ => Err(format_err!(lit, "Expected an integer literal")),
        },
        undefined => Err(format_err!(undefined, "Expected an integer literal")),
    }
}

/// Index type for the LUT
#[derive(Copy, Clone, Debug)]
pub enum ReprKind {
    /// Found a `u8` annotation.
    U8 = 8,
    /// Found a `u16` annotation.
    U16 = 16,
    /// Found a `u32` annotation.
    U32 = 32,
}

impl ToTokens for ReprKind {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            ReprKind::U8 => tokens.extend(quote!(u8)),
            ReprKind::U16 => tokens.extend(quote!(u16)),
            ReprKind::U32 => tokens.extend(quote!(u32)),
        }
    }
}

impl Parse for ReprKind {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let index_type: syn::TypePath = input
            .parse()
            .map_err(|e| e.into_combine(format_err!(input.span(), "Expected a u8/u16/u32")))?;

        index_type.path.try_into()
    }
}

impl TryFrom<syn::Path> for ReprKind {
    type Error = syn::Error;

    fn try_from(value: syn::Path) -> Result<Self, Self::Error> {
        if value.is_ident("u8") {
            Some(ReprKind::U8)
        } else if value.is_ident("u16") {
            Some(ReprKind::U16)
        } else if value.is_ident("u32") {
            Some(ReprKind::U32)
        } else {
            None
        }
        .ok_or_else(|| format_err!(value, "Unsupported index type, only `u8`, `u16`, or `u32` is supported"))
    }
}
