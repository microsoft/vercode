// Copyright (c) Microsoft Corporation. All rights reserved.
use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Meta, parse_macro_input};

/// Helper struct for field information during code generation
#[derive(Clone)]
struct FieldInfo<'a> {
    index: usize,
    version: u32,
    ty: &'a syn::Type,
    ident: Option<&'a syn::Ident>,
}

impl<'a> FieldInfo<'a> {
    fn temp_var(&self) -> syn::Ident {
        syn::Ident::new(
            &format!("__field{}", self.index),
            proc_macro2::Span::call_site(),
        )
    }
}

/// Batches of fields grouped by version number
struct VersionBatch<'a> {
    version: u32,
    fields: Vec<FieldInfo<'a>>,
}

fn parse_version_attribute(attrs: &[syn::Attribute]) -> u32 {
    for attr in attrs {
        if let Meta::List(list) = &attr.meta
            && list.path.is_ident("version")
        {
            let ts = list.tokens.to_string();
            let digits: String = ts.chars().filter(|c| c.is_ascii_digit()).collect();
            if let Ok(v) = digits.parse::<u32>() {
                return v;
            }
        }
    }
    0
}

fn field_version(field: &syn::Field) -> u32 {
    parse_version_attribute(&field.attrs)
}

fn variant_version(variant: &syn::Variant) -> u32 {
    parse_version_attribute(&variant.attrs)
}

/// Extract and organize field information from Fields
fn extract_field_info(fields: &Fields) -> Vec<FieldInfo<'_>> {
    match fields {
        Fields::Named(named) => named
            .named
            .iter()
            .enumerate()
            .map(|(i, f)| FieldInfo {
                index: i,
                version: field_version(f),
                ty: &f.ty,
                ident: f.ident.as_ref(),
            })
            .collect(),
        Fields::Unnamed(unnamed) => unnamed
            .unnamed
            .iter()
            .enumerate()
            .map(|(i, f)| FieldInfo {
                index: i,
                version: field_version(f),
                ty: &f.ty,
                ident: None,
            })
            .collect(),
        Fields::Unit => vec![],
    }
}

/// Sort fields by version, then by original index, and batch by version
fn create_version_batches(mut field_infos: Vec<FieldInfo>) -> Vec<VersionBatch> {
    // Sort by version first, then by original index
    field_infos.sort_by_key(|f| (f.version, f.index));

    // Group into batches by version
    let mut batches: Vec<VersionBatch> = Vec::new();
    for field in field_infos {
        if let Some(last_batch) = batches.last_mut()
            && last_batch.version == field.version
        {
            last_batch.fields.push(field);
            continue;
        }
        batches.push(VersionBatch {
            version: field.version,
            fields: vec![field],
        });
    }
    batches
}

/// Generate write code for a batch of fields
fn generate_field_writes(
    batches: &[VersionBatch],
    is_named: bool,
) -> Vec<proc_macro2::TokenStream> {
    let mut writes = Vec::new();
    let mut last_version = 0u32;

    for batch in batches {
        if batch.version != last_version {
            last_version = batch.version;
            let v = batch.version;
            writes.push(quote! { if version < #v { return offset; } });
        }

        for field in &batch.fields {
            let write_stmt = if is_named {
                let ident = field.ident.unwrap();
                quote! { offset += ::vercode::VerCodable::write_version(&self.#ident, version, &mut buf[offset..]); }
            } else {
                let idx = syn::Index::from(field.index);
                quote! { offset += ::vercode::VerCodable::write_version(&self.#idx, version, &mut buf[offset..]); }
            };
            writes.push(write_stmt);
        }
    }
    writes
}

/// Generate size calculation code for a batch of fields
fn generate_field_sizes(batches: &[VersionBatch], is_named: bool) -> Vec<proc_macro2::TokenStream> {
    let mut sizes = Vec::new();
    let mut last_version = 0u32;

    for batch in batches {
        if batch.version != last_version {
            last_version = batch.version;
            let v = batch.version;
            sizes.push(quote! { if version < #v { return total; } });
        }

        for field in &batch.fields {
            let size_stmt = if is_named {
                let ident = field.ident.unwrap();
                quote! { total += ::vercode::VerCodable::size_version(&self.#ident, version); }
            } else {
                let idx = syn::Index::from(field.index);
                quote! { total += ::vercode::VerCodable::size_version(&self.#idx, version); }
            };
            sizes.push(size_stmt);
        }
    }
    sizes
}

/// Generate read code for version batches
fn generate_field_reads(batches: &[VersionBatch]) -> Vec<proc_macro2::TokenStream> {
    let mut reads = Vec::new();

    for batch in batches {
        let temp_vars: Vec<_> = batch.fields.iter().map(|f| f.temp_var()).collect();
        let mut read_stmts = Vec::new();
        let mut default_stmts = Vec::new();

        for field in &batch.fields {
            let temp_var = field.temp_var();
            let ty = field.ty;
            read_stmts.push(quote! {
                (#temp_var, __temp_size) = <#ty as ::vercode::VerCodable>::read_version(version, &buf[offset..])?;
                offset += __temp_size;
            });
            default_stmts.push(quote! {
                #temp_var = <#ty as ::std::default::Default>::default();
            });
        }

        if batch.version == 0 {
            // Version 0 fields always read
            reads.push(quote! {
                #(let mut #temp_vars;)*
                let mut __temp_size;
                #(#read_stmts)*
            });
        } else {
            let v = batch.version;
            reads.push(quote! {
                #(let mut #temp_vars;)*
                let mut __temp_size;
                if offset < length && version >= #v {
                    #(#read_stmts)*
                } else {
                    #(#default_stmts)*
                }
            });
        }
    }
    reads
}

/// Generate struct construction from field info
fn generate_struct_construction(
    name: &syn::Ident,
    fields: &Fields,
    field_infos: &[FieldInfo],
) -> proc_macro2::TokenStream {
    match fields {
        Fields::Named(_) => {
            let field_inits: Vec<_> = field_infos
                .iter()
                .map(|f| {
                    let ident = f.ident.unwrap();
                    let temp_var = f.temp_var();
                    quote! { #ident: #temp_var }
                })
                .collect();
            quote! { #name { #(#field_inits),* } }
        }
        Fields::Unnamed(_) => {
            let field_values: Vec<_> = field_infos.iter().map(|f| f.temp_var()).collect();
            quote! { #name ( #(#field_values),* ) }
        }
        Fields::Unit => quote! { #name },
    }
}

/// Calculate maximum version expression from field infos
fn calculate_max_version_expr(field_infos: &[FieldInfo]) -> proc_macro2::TokenStream {
    // Calculate max from field version attributes
    let field_attr_max = field_infos.iter().map(|f| f.version).max().unwrap_or(0);

    // Generate expressions for each field type's MAX_VERSION
    let field_type_exprs: Vec<_> = field_infos
        .iter()
        .map(|f| {
            let ty = f.ty;
            quote! { <#ty as ::vercode::VerCodable>::MAX_VERSION }
        })
        .collect();

    // Generate a const expression that computes the max of all versions
    if field_type_exprs.is_empty() {
        quote! { #field_attr_max }
    } else {
        quote! {
            {
                let mut max = #field_attr_max;
                #(
                    if #field_type_exprs > max {
                        max = #field_type_exprs;
                    }
                )*
                max
            }
        }
    }
}

/// Variant information for enum processing
struct VariantInfo<'a> {
    index: usize,
    variant: &'a syn::Variant,
    field_infos: Vec<FieldInfo<'a>>,
    batches: Vec<VersionBatch<'a>>,
}

impl<'a> VariantInfo<'a> {
    fn new(index: usize, variant: &'a syn::Variant) -> Self {
        let field_infos = extract_field_info(&variant.fields);
        let batches = create_version_batches(field_infos.clone());
        VariantInfo {
            index,
            variant,
            field_infos,
            batches,
        }
    }

    fn max_version_expr(&self) -> proc_macro2::TokenStream {
        let variant_ver = variant_version(self.variant);

        // Get field attribute versions
        let field_attr_max = self
            .field_infos
            .iter()
            .map(|f| f.version)
            .max()
            .unwrap_or(0);

        // Generate expressions for each field type's MAX_VERSION
        let field_type_exprs: Vec<_> = self
            .field_infos
            .iter()
            .map(|f| {
                let ty = f.ty;
                quote! { <#ty as ::vercode::VerCodable>::MAX_VERSION }
            })
            .collect();

        // Generate a const expression that computes the max of all versions
        if field_type_exprs.is_empty() {
            let max = variant_ver.max(field_attr_max);
            quote! { #max }
        } else {
            quote! {
                {
                    let mut max = #variant_ver;
                    if #field_attr_max > max {
                        max = #field_attr_max;
                    }
                    #(
                        if #field_type_exprs > max {
                            max = #field_type_exprs;
                        }
                    )*
                    max
                }
            }
        }
    }

    /// Generate pattern match binding for this variant
    fn match_pattern(&self, enum_name: &syn::Ident) -> proc_macro2::TokenStream {
        let var_name = &self.variant.ident;
        match &self.variant.fields {
            Fields::Named(_) => {
                let actual_names: Vec<_> =
                    self.field_infos.iter().map(|f| f.ident.unwrap()).collect();
                let temp_vars: Vec<_> = self.field_infos.iter().map(|f| f.temp_var()).collect();
                quote! { #enum_name::#var_name { #(#actual_names: #temp_vars),* } }
            }
            Fields::Unnamed(_) => {
                let temp_vars: Vec<_> = self.field_infos.iter().map(|f| f.temp_var()).collect();
                quote! { #enum_name::#var_name(#(#temp_vars),*) }
            }
            Fields::Unit => quote! { #enum_name::#var_name },
        }
    }

    /// Generate variant construction from temp variables
    fn construct_variant(&self, enum_name: &syn::Ident) -> proc_macro2::TokenStream {
        let var_name = &self.variant.ident;
        match &self.variant.fields {
            Fields::Named(_) => {
                let actual_names: Vec<_> =
                    self.field_infos.iter().map(|f| f.ident.unwrap()).collect();
                let temp_vars: Vec<_> = self.field_infos.iter().map(|f| f.temp_var()).collect();
                quote! { #enum_name::#var_name { #(#actual_names: #temp_vars),* } }
            }
            Fields::Unnamed(_) => {
                let temp_vars: Vec<_> = self.field_infos.iter().map(|f| f.temp_var()).collect();
                quote! { #enum_name::#var_name(#(#temp_vars),*) }
            }
            Fields::Unit => quote! { #enum_name::#var_name },
        }
    }

    /// Generate write arm for this variant
    fn write_arm(&self, enum_name: &syn::Ident) -> proc_macro2::TokenStream {
        let idx_u32 = self.index as u32;
        let pattern = self.match_pattern(enum_name);
        let field_writes = generate_variant_field_writes(&self.batches);

        quote! {
            #pattern => {
                buf[offset..offset+2].copy_from_slice(&(#idx_u32 as u16).to_le_bytes());
                offset += 2;
                #(#field_writes)*
            }
        }
    }

    /// Generate size arm for this variant
    fn size_arm(&self, enum_name: &syn::Ident) -> proc_macro2::TokenStream {
        let pattern = self.match_pattern(enum_name);
        let field_sizes = generate_variant_field_sizes(&self.batches);

        quote! {
            #pattern => {
                #(#field_sizes)*
            }
        }
    }

    /// Generate read arm for this variant
    fn read_arm(&self, enum_name: &syn::Ident) -> proc_macro2::TokenStream {
        let idx_u32 = self.index as u32;
        let reads = generate_field_reads(&self.batches);
        let construction = self.construct_variant(enum_name);

        quote! {
            #idx_u32 => {
                #(#reads)*
                #construction
            }
        }
    }
}

/// Generate write statements for variant fields (using temp vars)
fn generate_variant_field_writes(batches: &[VersionBatch]) -> Vec<proc_macro2::TokenStream> {
    let mut writes = Vec::new();
    let mut last_version = 0u32;

    for batch in batches {
        if batch.version != last_version {
            last_version = batch.version;
            let v = batch.version;
            writes.push(quote! { if version < #v { return offset; } });
        }

        for field in &batch.fields {
            let temp_var = field.temp_var();
            writes.push(quote! {
                offset += ::vercode::VerCodable::write_version(#temp_var, version, &mut buf[offset..]);
            });
        }
    }
    writes
}

/// Generate size statements for variant fields (using temp vars)
fn generate_variant_field_sizes(batches: &[VersionBatch]) -> Vec<proc_macro2::TokenStream> {
    let mut sizes = Vec::new();
    let mut last_version = 0u32;

    for batch in batches {
        if batch.version != last_version {
            last_version = batch.version;
            let v = batch.version;
            sizes.push(quote! { if version < #v { return total; } });
        }

        for field in &batch.fields {
            let temp_var = field.temp_var();
            sizes.push(quote! {
                total += ::vercode::VerCodable::size_version(#temp_var, version);
            });
        }
    }
    sizes
}

#[proc_macro_derive(VercodeTransparent)]
pub fn derive_vercode_transparent(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Verify it's a newtype struct and get field accessor
    let (inner_type, field_accessor, construction) = match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                let ty = &fields.unnamed.first().unwrap().ty;
                (ty, quote! { 0 }, quote! { Self(inner) })
            }
            Fields::Named(fields) if fields.named.len() == 1 => {
                let field = fields.named.first().unwrap();
                let ty = &field.ty;
                let field_name = field.ident.as_ref().unwrap();
                (
                    ty,
                    quote! { #field_name },
                    quote! { Self { #field_name: inner } },
                )
            }
            _ => panic!(
                "VercodeTransparent can only be used on newtype structs with exactly one field"
            ),
        },
        _ => panic!("VercodeTransparent can only be used on structs"),
    };

    let expanded = quote! {
        impl #impl_generics ::vercode::VerCodable for #name #ty_generics #where_clause {
            const MAX_VERSION: u32 = <#inner_type as ::vercode::VerCodable>::MAX_VERSION;

            #[inline(always)]
            fn write_version(&self, version: u32, buf: &mut [u8]) -> usize {
                ::vercode::VerCodable::write_version(&self.#field_accessor, version, buf)
            }

            #[inline(always)]
            fn read_version(version: u32, buf: &[u8]) -> ::std::result::Result<(Self, usize), ::vercode::InvalidEncoding> {
                let (inner, size) = <#inner_type as ::vercode::VerCodable>::read_version(version, buf)?;
                Ok((#construction, size))
            }

            #[inline(always)]
            fn size_version(&self, version: u32) -> usize {
                ::vercode::VerCodable::size_version(&self.#field_accessor, version)
            }

            #[inline(always)]
            fn write_option(this: Option<&Self>, version: u32, buf: &mut [u8]) -> usize {
                ::vercode::VerCodable::write_option(
                    this.map(|this| &this.#field_accessor),
                    version,
                    buf,
                )
            }

            #[inline(always)]
            fn read_option(version: u32, buf: &[u8]) -> Result<(Option<Self>, usize), ::vercode::InvalidEncoding> {
                let (inner_option, size) = ::vercode::VerCodable::read_option(version, buf)?;
                let result_option = inner_option.map(|inner| #construction);
                Ok((result_option, size))
            }

            #[inline(always)]
            fn size_option_version(this: &Option<Self>, version: u32) -> usize {
                let inner_option = this.as_ref().map(|this| this.#field_accessor);
                <#inner_type as ::vercode::VerCodable>::size_option_version(&inner_option, version)
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(Vercode, attributes(version))]
pub fn derive_vercode(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    match &input.data {
        Data::Struct(s) => {
            derive_struct(name, &impl_generics, &ty_generics, &where_clause, &s.fields)
        }
        Data::Enum(e) => derive_enum(
            name,
            &impl_generics,
            &ty_generics,
            &where_clause,
            &e.variants,
        ),
        _ => panic!("Vercode only supports structs and enums"),
    }
}

fn derive_struct(
    name: &syn::Ident,
    impl_generics: &syn::ImplGenerics,
    ty_generics: &syn::TypeGenerics,
    where_clause: &Option<&syn::WhereClause>,
    fields: &Fields,
) -> TokenStream {
    let field_infos = extract_field_info(fields);
    let batches = create_version_batches(field_infos.clone());

    let max_version_expr = calculate_max_version_expr(&field_infos);

    let is_named = matches!(fields, Fields::Named(_));
    let writes = generate_field_writes(&batches, is_named);
    let sizes = generate_field_sizes(&batches, is_named);
    let reads = generate_field_reads(&batches);
    let construction = generate_struct_construction(name, fields, &field_infos);

    let expanded = quote! {
        impl #impl_generics ::vercode::VerCodable for #name #ty_generics #where_clause {
            const MAX_VERSION: u32 = #max_version_expr;

            #[inline(always)]
            fn write_version(&self, version: u32, buf: &mut [u8]) -> usize {
                let total_data = self.size_version(version);
                buf[..4].copy_from_slice(&(total_data as u32).to_le_bytes());
                let mut offset = 4usize;
                #(#writes)*
                offset
            }

            #[inline(always)]
            fn read_version(version: u32, buf: &[u8]) -> ::std::result::Result<(Self, usize), ::vercode::InvalidEncoding> {
                if buf.len() < 4 { return Err(::vercode::InvalidEncoding); }
                let length = u32::from_le_bytes(buf[..4].try_into().unwrap()) as usize;
                let mut offset = 4usize;
                #(#reads)*
                let result = #construction;
                Ok((result, offset))
            }

            #[inline(always)]
            fn size_version(&self, version: u32) -> usize {
                let mut total = 4usize;
                #(#sizes)*
                total
            }
        }
    };
    TokenStream::from(expanded)
}

fn derive_enum(
    name: &syn::Ident,
    impl_generics: &syn::ImplGenerics,
    ty_generics: &syn::TypeGenerics,
    where_clause: &Option<&syn::WhereClause>,
    variants: &syn::punctuated::Punctuated<syn::Variant, syn::token::Comma>,
) -> TokenStream {
    // Process all variants once
    let variant_infos: Vec<VariantInfo> = variants
        .iter()
        .enumerate()
        .map(|(idx, variant)| VariantInfo::new(idx, variant))
        .collect();

    // Calculate max version expression
    // Generate const expression that computes max across all variants
    let variant_max_exprs: Vec<_> = variant_infos.iter().map(|v| v.max_version_expr()).collect();

    let max_version_expr = if variant_max_exprs.is_empty() {
        quote! { 0 }
    } else {
        quote! {
            {
                let mut max = 0;
                #(
                    {
                        let variant_max = #variant_max_exprs;
                        if variant_max > max {
                            max = variant_max;
                        }
                    }
                )*
                max
            }
        }
    };

    // Generate match arms using the processed variant info
    let write_arms: Vec<_> = variant_infos.iter().map(|v| v.write_arm(name)).collect();
    let size_arms: Vec<_> = variant_infos.iter().map(|v| v.size_arm(name)).collect();
    let read_arms: Vec<_> = variant_infos.iter().map(|v| v.read_arm(name)).collect();

    let expanded = quote! {
        impl #impl_generics ::vercode::VerCodable for #name #ty_generics #where_clause {
            const MAX_VERSION: u32 = #max_version_expr;

            #[inline(always)]
            fn write_version(&self, version: u32, buf: &mut [u8]) -> usize {
                let total_data = self.size_version(version);
                buf[..4].copy_from_slice(&(total_data as u32).to_le_bytes());
                let mut offset = 4usize;
                match self {
                    #(#write_arms)*
                }
                offset
            }

            #[inline(always)]
            fn read_version(version: u32, buf: &[u8]) -> ::std::result::Result<(Self, usize), ::vercode::InvalidEncoding> {
                if buf.len() < 6 { return Err(::vercode::InvalidEncoding); }
                let length = u32::from_le_bytes(buf[..4].try_into().unwrap()) as usize;
                let discriminant = u16::from_le_bytes(buf[4..6].try_into().unwrap()) as u32;
                let mut offset = 6usize;

                let result = match discriminant {
                    #(#read_arms,)*
                    _ => return Err(::vercode::InvalidEncoding),
                };
                Ok((result, offset))
            }

            #[inline(always)]
            fn size_version(&self, version: u32) -> usize {
                let mut total = 6usize; // length prefix (4 bytes) + discriminant (2 bytes)
                match self {
                    #(#size_arms)*
                }
                total
            }
        }
    };

    TokenStream::from(expanded)
}
