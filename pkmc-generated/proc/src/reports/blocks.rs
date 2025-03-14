use std::collections::{BTreeMap, BTreeSet};

use convert_case::Casing;
use proc_macro2::{Literal, TokenStream};
use quote::{quote, ToTokens};
use serde::Deserialize;
use syn::{parse::Parse, parse_macro_input, spanned::Spanned as _, Ident, LitStr};

use crate::{file_path, fix_identifier};

struct ReportBlocksEnum {
    file: LitStr,
}

impl Parse for ReportBlocksEnum {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            file: input.parse()?,
        })
    }
}

impl ToTokens for ReportBlocksEnum {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        #[derive(Deserialize)]
        struct ReportBlockState {
            id: i32,
            #[serde(default)]
            default: bool,
            #[serde(default)]
            properties: BTreeMap<String, String>,
        }

        #[derive(Deserialize)]
        struct ReportBlock {
            definition: serde_json::Value,
            #[serde(default)]
            properties: BTreeMap<String, BTreeSet<String>>,
            states: Vec<ReportBlockState>,
        }

        #[derive(Deserialize)]
        struct ReportBlocks(BTreeMap<String, ReportBlock>);

        let file = file_path(&self.file.value());

        let blocks_report: ReportBlocks =
            serde_json::from_reader(std::fs::File::open(&file).expect("Failed to open file"))
                .expect("Failed to parse JSON");

        let mut blocks_tokens = TokenStream::new();

        #[derive(PartialEq, Eq, Clone)]
        enum PropertyType {
            Boolean,
            Enum(BTreeSet<String>),
        }

        impl From<BTreeSet<String>> for PropertyType {
            fn from(value: BTreeSet<String>) -> Self {
                if value == BTreeSet::from(["true".to_owned(), "false".to_owned()]) {
                    PropertyType::Boolean
                } else {
                    PropertyType::Enum(value)
                }
            }
        }

        let mut properties_enums: BTreeMap<String, Vec<BTreeSet<String>>> = BTreeMap::new();

        blocks_report.0.into_iter().for_each(|(identifier, data)| {
            let properties = data
                .properties
                .into_iter()
                .map(|(name, values)| {
                    (
                        if matches!(name.as_str(), "type") {
                            Ident::new_raw(&name, blocks_tokens.span())
                        } else {
                            Ident::new(&name, blocks_tokens.span())
                        },
                        match PropertyType::from(values) {
                            PropertyType::Boolean => quote! { bool },
                            PropertyType::Enum(values) => {
                                let properties_with_same_name =
                                    properties_enums.entry(name.clone()).or_default();
                                if !properties_with_same_name.contains(&values) {
                                    properties_with_same_name.push(values.clone());
                                }
                                if let Some((index, _)) = properties_with_same_name
                                    .iter()
                                    .enumerate()
                                    .find(|(_, v)| *v == &values)
                                {
                                    let name = Ident::new(
                                        &format!("{}_{}", name, index)
                                            .to_case(convert_case::Case::Pascal),
                                        blocks_tokens.span(),
                                    );
                                    quote! { #name }
                                } else {
                                    unreachable!()
                                }
                            }
                        },
                    )
                })
                .collect::<Vec<_>>();

            let name = Ident::new(&fix_identifier(&identifier), blocks_tokens.span());
            if properties.is_empty() {
                blocks_tokens.extend(quote! {
                    #name,
                });
            } else {
                let (property_name, property_type) =
                    properties.into_iter().collect::<(Vec<_>, Vec<_>)>();
                blocks_tokens.extend(quote! {
                    #name {
                        #(#property_name: #property_type,)*
                    },
                });
            }
        });

        properties_enums
            .into_iter()
            .for_each(|(name, enum_values)| {
                for (index, values) in enum_values.into_iter().enumerate() {
                    let name = Ident::new(
                        &format!("{}_{}", name, index).to_case(convert_case::Case::Pascal),
                        blocks_tokens.span(),
                    );
                    let values = values
                        .into_iter()
                        .map(|v| {
                            let v = v.to_case(convert_case::Case::Pascal);
                            if !v.chars().next().unwrap().is_numeric() {
                                Ident::new(&v, tokens.span())
                            } else {
                                Ident::new(&format!("N{}", v), tokens.span())
                            }
                        })
                        .collect::<Vec<_>>();
                    let indices = values
                        .iter()
                        .enumerate()
                        .map(|(i, _)| Literal::usize_unsuffixed(i))
                        .collect::<Vec<_>>();
                    tokens.extend(quote! {
                        pub enum #name {
                            #(#values,)*
                        }

                        impl #name {
                            pub fn to_index(&self) -> usize {
                                match self {
                                    #(Self::#values => #indices,)*
                                }
                            }

                            pub fn from_index(index: usize) -> Option<Self> {
                                match index {
                                    #(#indices => Some(Self::#values),)*
                                    _ => None,
                                }
                            }
                        }
                    });
                }
            });

        tokens.extend(quote! {
            pub enum Block {
                #blocks_tokens
            }

            impl Block {

            }
        });
    }
}

pub(crate) fn report_blocks_generate_enum(
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let c = parse_macro_input!(input as ReportBlocksEnum);
    quote! { #c }.into()
}
