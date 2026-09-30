// Copyright (c) Microsoft Corporation. All rights reserved.
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Meta, Token, punctuated::Punctuated};

pub(crate) fn derive(input: DeriveInput, repr: &str, marker: &str) -> syn::Result<TokenStream> {
    let Data::Enum(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            &input.ident,
            format!("{marker} only supports fieldless enums"),
        ));
    };
    for variant in &data.variants {
        if !matches!(variant.fields, Fields::Unit) {
            return Err(syn::Error::new_spanned(
                &variant.fields,
                format!("{marker} only supports fieldless enums"),
            ));
        }
    }

    let mut has_repr = false;
    for attr in &input.attrs {
        if attr.path().is_ident("repr") {
            let metas = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
            has_repr |= metas.iter().any(|meta| meta.path().is_ident(repr));
        }
    }
    if !has_repr {
        return Err(syn::Error::new_spanned(
            &input.ident,
            format!("{marker} requires #[repr({repr})]"),
        ));
    }

    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let repr = syn::Ident::new(repr, proc_macro2::Span::call_site());
    let marker = syn::Ident::new(marker, proc_macro2::Span::call_site());

    Ok(quote! {
        impl #impl_generics ::vercode::#marker for #name #ty_generics #where_clause {}

        impl #impl_generics ::vercode::VerCodable for #name #ty_generics #where_clause {
            const MAX_VERSION: u32 = <#repr as ::vercode::VerCodable>::MAX_VERSION;

            #[inline(always)]
            fn write_version(&self, version: u32, buf: &mut [u8]) -> usize {
                let raw: #repr = ::core::convert::Into::into(*self);
                <#repr as ::vercode::VerCodable>::write_version(&raw, version, buf)
            }

            #[inline(always)]
            fn read_version(version: u32, buf: &[u8]) -> ::core::result::Result<(Self, usize), ::vercode::InvalidEncoding> {
                // Integer readers assume sufficient input; reject truncation before delegating.
                if buf.len() < ::core::mem::size_of::<#repr>() {
                    return Err(::vercode::InvalidEncoding);
                }
                let (raw, size) = <#repr as ::vercode::VerCodable>::read_version(version, buf)?;
                let value = <Self as ::core::convert::TryFrom<#repr>>::try_from(raw)
                    .unwrap_or_default();
                Ok((value, size))
            }

            #[inline(always)]
            fn size_version(&self, version: u32) -> usize {
                let raw: #repr = ::core::convert::Into::into(*self);
                <#repr as ::vercode::VerCodable>::size_version(&raw, version)
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn accepts_matching_representations() {
        assert!(
            derive(
                parse_quote!(
                    #[repr(u8)]
                    enum Mode {
                        Normal,
                    }
                ),
                "u8",
                "U8Enum"
            )
            .is_ok()
        );
        assert!(
            derive(
                parse_quote!(
                    #[repr(u16)]
                    enum Mode {
                        Normal,
                    }
                ),
                "u16",
                "U16Enum"
            )
            .is_ok()
        );
    }

    #[test]
    fn rejects_missing_or_mismatched_representation() {
        for input in [
            parse_quote!(
                enum Mode {
                    Normal,
                }
            ),
            parse_quote!(
                #[repr(u16)]
                enum Mode {
                    Normal,
                }
            ),
        ] {
            let error = derive(input, "u8", "U8Enum").unwrap_err();
            assert_eq!(error.to_string(), "U8Enum requires #[repr(u8)]");
        }
        let error = derive(
            parse_quote!(
                #[repr(u8)]
                enum Mode {
                    Normal,
                }
            ),
            "u16",
            "U16Enum",
        )
        .unwrap_err();
        assert_eq!(error.to_string(), "U16Enum requires #[repr(u16)]");
    }

    #[test]
    fn rejects_non_fieldless_enums() {
        for input in [
            parse_quote!(
                struct Mode(u8);
            ),
            parse_quote!(
                #[repr(u8)]
                enum Mode {
                    Data(u8),
                }
            ),
            parse_quote!(
                #[repr(u8)]
                enum Mode {
                    Data { value: u8 },
                }
            ),
        ] {
            let error = derive(input, "u8", "U8Enum").unwrap_err();
            assert_eq!(error.to_string(), "U8Enum only supports fieldless enums");
        }
    }
}
