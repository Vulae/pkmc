/*
    TODO:
        Cleanup all of this! I know it's very bad right now, I'm very sorry for that.
        Add ability to group certain block types (Eg. Woods & colored blocks with an extra property with enum) to reduce code & enum size.
*/

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use convert_case::Casing;
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use serde::Deserialize;
use syn::{
    Ident, LitStr, Token, parse::Parse, parse_macro_input, punctuated::Punctuated,
    spanned::Spanned as _,
};

use crate::{file_path, fix_identifier};

#[derive(Deserialize)]
#[allow(unused)]
struct ReportBlockState {
    id: i32,
    #[serde(default)]
    default: bool,
    #[serde(default)]
    properties: BTreeMap<String, String>,
}

#[derive(Deserialize)]
#[allow(unused)]
struct ReportBlockDefintion {
    r#type: String,
    #[serde(flatten)]
    rest: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
struct ReportBlock {
    definition: ReportBlockDefintion,
    #[serde(default)]
    properties: BTreeMap<String, Vec<String>>,
    states: Vec<ReportBlockState>,
}

type ReportBlocks = BTreeMap<String, ReportBlock>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum PropertyType {
    Boolean,
    Number(u32),
    Enum(Vec<String>),
}

impl From<Vec<String>> for PropertyType {
    fn from(value: Vec<String>) -> Self {
        assert!(value.len() > 1, "PropertyType invalid number of states");
        if value == vec!["true".to_owned(), "false".to_owned()] {
            PropertyType::Boolean
        } else if let Some(count) = value
            .iter()
            .map(|v| v.parse::<u32>().ok())
            .collect::<Option<Vec<u32>>>()
            .and_then(|mut nums| {
                nums.sort();
                match nums.first().unwrap() {
                    0 => nums
                        .iter()
                        .enumerate()
                        .all(|(i, v)| i as u32 == *v)
                        .then_some(nums.len() - 1),
                    1 => nums
                        .iter()
                        .enumerate()
                        .all(|(i, v)| (i as u32 + 1) == *v)
                        .then_some(nums.len() - 1),
                    _ => panic!(),
                }
            })
        {
            PropertyType::Number(count as u32)
        } else {
            PropertyType::Enum(value)
        }
    }
}

type PropertiesMap = BTreeMap<String, BTreeSet<PropertyType>>;

struct ReportBlocksGenerator {
    blocks_report: ReportBlocks,
    mapper: HashMap<Ident, Ident>,
}

impl ReportBlocksGenerator {
    fn generate_properties(&self) -> PropertiesMap {
        let mut properties = PropertiesMap::new();
        self.blocks_report
            .iter()
            .for_each(|(_block_name, block_data)| {
                block_data
                    .properties
                    .iter()
                    .for_each(|(property_name, property_values)| {
                        let property_name = property_name.clone();
                        let property_type = PropertyType::from(property_values.clone());
                        properties
                            .entry(property_name)
                            .or_default()
                            .insert(property_type);
                    });
            });
        properties
    }

    fn generate_properties_code(&self, map: &PropertiesMap, tokens: &mut proc_macro2::TokenStream) {
        let mut already_generated: HashSet<Ident> = HashSet::new();
        map.iter().for_each(|(property_name, property_types)| {
            property_types
                .iter()
                .enumerate()
                .for_each(|(i, property_type)| {
                    if let PropertyType::Enum(vec) = property_type {
                        let mut enum_name = Ident::new(
                            &format!("{}{}", property_name, i).to_case(convert_case::Case::Pascal),
                            tokens.span(),
                        );
                        if let Some(mapped_name) = self.mapper.get(&enum_name) {
                            enum_name = mapped_name.clone();
                        }
                        if !already_generated.insert(enum_name.clone()) {
                            return;
                        }
                        let enum_values = vec
                            .iter()
                            .map(|v| {
                                Ident::new(&v.to_case(convert_case::Case::Pascal), tokens.span())
                            })
                            .collect::<Vec<_>>();
                        let num_values = vec.len() as u32;
                        let enum_indices = (0..vec.len()).map(|v| v as u32).collect::<Vec<_>>();
                        tokens.extend(quote! {
                            #[derive(Debug, Clone, Copy)]
                            pub enum #enum_name {
                                #(#enum_values,)*
                            }

                            impl IdIndexable for #enum_name {
                                const NUM_STATES: u32 = #num_values;

                                fn into_index(self) -> u32 {
                                    match self {
                                        #(Self::#enum_values => #enum_indices,)*
                                    }
                                }

                                fn from_index(index: u32) -> Option<Self> {
                                    match index {
                                        #(#enum_indices => Some(Self::#enum_values),)*
                                        _ => None,
                                    }
                                }
                            }
                        });
                    }
                });
        });
    }

    fn generate_blocks_code_enum(
        &self,
        properties_map: &PropertiesMap,
        tokens: &mut proc_macro2::TokenStream,
    ) {
        let mut blocks_tokens = proc_macro2::TokenStream::new();
        let mut blocks_to_id_tokens = proc_macro2::TokenStream::new();
        let mut blocks_from_id_tokens = proc_macro2::TokenStream::new();

        self.blocks_report.iter().for_each(|(name, def)| {
            let name = Ident::new(&fix_identifier(name), tokens.span());
            let id = def.states.first().unwrap().id as u32;
            def.states.iter().enumerate().for_each(|(i, state)| {
                assert_eq!(id + i as u32, state.id as u32);
            });

            if def.properties.is_empty() {
                blocks_tokens.extend(quote! {
                    #name,
                });
                blocks_to_id_tokens.extend(quote! {
                    Self::#name => #id,
                });
                blocks_from_id_tokens.extend(quote! {
                    #id => Self::#name,
                });
                return;
            }

            struct ParsedProperty<'a> {
                ident_name: Ident,
                ident_type: TokenStream,
                r#type: &'a PropertyType,
            }

            let parsed_props: Vec<ParsedProperty> = def
                .properties
                .iter()
                .map(|(property_name, property_values)| {
                    let (i, r#type) = properties_map
                        .get(property_name)
                        .unwrap()
                        .iter()
                        .enumerate()
                        .find(|(_, v)| *v == &PropertyType::from(property_values.clone()))
                        .unwrap();
                    ParsedProperty {
                        ident_name: match property_name.as_ref() {
                            "type" => Ident::new_raw(property_name, tokens.span()),
                            _ => Ident::new(property_name, tokens.span()),
                        },
                        ident_type: match r#type {
                            PropertyType::Boolean => quote! { bool },
                            PropertyType::Number(max) => quote! { PropertyUint::<#max> },
                            PropertyType::Enum(_) => {
                                let mut enum_name = Ident::new(
                                    &format!("{}{}", property_name, i)
                                        .to_case(convert_case::Case::Pascal),
                                    tokens.span(),
                                );
                                if let Some(mapped_name) = self.mapper.get(&enum_name) {
                                    enum_name = mapped_name.clone();
                                }
                                quote! { #enum_name }
                            }
                        },
                        r#type,
                    }
                })
                .collect();

            let property_names = parsed_props
                .iter()
                .map(|v| v.ident_name.clone())
                .collect::<Vec<_>>();
            let property_types = parsed_props
                .iter()
                .map(|v| v.ident_type.clone())
                .collect::<Vec<_>>();
            blocks_tokens.extend(quote! {
                #name {
                    #(#property_names: #property_types,)*
                },
            });

            let test = parsed_props.iter().enumerate().fold(quote! { }, |inner, (i, parsed)| {
                let name = &parsed.ident_name;
                let r#type = &parsed.ident_type;
                if i == 0 {
                    match parsed.r#type {
                        PropertyType::Boolean => {
                            quote! { (!#name as u32) }
                        },
                        PropertyType::Number(_) | PropertyType::Enum(_) => {
                            quote! { #name.into_index() }
                        },
                    }
                } else {
                    match parsed.r#type {
                        PropertyType::Boolean => {
                            quote! { (#inner * 2 + (!#name as u32)) }
                        },
                        PropertyType::Number(_) | PropertyType::Enum(_) => {
                            quote! { (#inner * #r#type::NUM_STATES + #name.into_index()) }
                        },
                    }
                }
            });

            blocks_to_id_tokens.extend(quote! {
                Self::#name {
                    #(#property_names,)*
                } => #id + #test,
            });

            let res1 = parsed_props
                .iter()
                .rev()
                .map(|parsed| {
                    let name = &parsed.ident_name;
                    let r#type = &parsed.ident_type;
                    match parsed.r#type {
                        PropertyType::Boolean => {
                            quote! {
                                let #name = v % 2 == 0;
                            }
                        }
                        PropertyType::Number(_) => {
                            quote! {
                                let #name = PropertyUint::from_index(v % #r#type::NUM_STATES).unwrap();
                            }
                        }
                        PropertyType::Enum(_) => {
                            quote! {
                                let #name = #r#type::from_index(v % #r#type::NUM_STATES).unwrap();
                            }
                        }
                    }
                })
                .collect::<Vec<_>>();
            let res2 = parsed_props.iter().skip(1).rev().map(|parsed| {
                let r#type = &parsed.ident_type;
                match parsed.r#type {
                    PropertyType::Boolean => {
                        quote! {
                            let v = v / 2;
                        }
                    }
                    PropertyType::Number(_) | PropertyType::Enum(_) => {
                        quote! {
                            let v = v / #r#type::NUM_STATES;
                        }
                    }
                }
            }).chain(std::iter::once(quote! {})).collect::<Vec<_>>();
            let res3 = parsed_props.iter().map(|parsed| &parsed.ident_name).collect::<Vec<_>>();

            let max_id = def.states.last().unwrap().id as u32;

            blocks_from_id_tokens.extend(quote! {
                #id..=#max_id => {
                    let v = id - #id;
                    #(
                        #res1
                        #res2
                    )*
                    Self::#name {
                        #(#res3,)*
                    }
                },
            });
        });

        tokens.extend(quote! {
            #[derive(Debug, Clone, Copy)]
            pub enum Block {
                #blocks_tokens
            }

            impl Block {
                pub fn into_id(self) -> i32 {
                    let v: u32 = match self {
                        #blocks_to_id_tokens
                    };
                    v as i32
                }

                pub fn from_id(id: i32) -> Option<Self> {
                    let id = id as u32;
                    Some(match id {
                        #blocks_from_id_tokens
                        _ => return None,
                    })
                }
            }
        });
    }

    fn generate_blocks_code_extra(&self, tokens: &mut proc_macro2::TokenStream) {
        let mut blocks_types = proc_macro2::TokenStream::new();

        self.blocks_report.iter().for_each(|(name, def)| {
            let name = Ident::new(&fix_identifier(name), blocks_types.span());
            let def_type = Ident::new(&fix_identifier(&def.definition.r#type), blocks_types.span());

            // TODO: Merge same types to single match arm.

            if def.properties.is_empty() {
                blocks_types.extend(quote! {
                    Self::#name => BlockType::#def_type,
                });
            } else {
                blocks_types.extend(quote! {
                    Self::#name { .. } => BlockType::#def_type,
                });
            }
        });

        tokens.extend(quote! {
            impl Block {
                pub const fn definition_type(&self) -> BlockType {
                    match self {
                        #blocks_types
                    }
                }
            }
        });
    }
}

struct ReportBlocksMapping {
    from: Ident,
    to: Ident,
}

impl Parse for ReportBlocksMapping {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            from: input.parse()?,
            to: {
                input.parse::<Token![=>]>()?;
                input.parse()?
            },
        })
    }
}

struct ReportBlocksEnum {
    file: LitStr,
    enums_mapper: Punctuated<ReportBlocksMapping, Token![,]>,
}

impl Parse for ReportBlocksEnum {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            file: input.parse()?,
            enums_mapper: {
                input.parse::<syn::Token![,]>()?;
                let content;
                syn::bracketed!(content in input);
                Punctuated::parse_terminated(&content)?
            },
        })
    }
}

impl ToTokens for ReportBlocksEnum {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let file = file_path(&self.file.value());

        let blocks_report: ReportBlocks =
            serde_json::from_reader(std::fs::File::open(&file).expect("Failed to open file"))
                .expect("Failed to parse JSON");

        let generator = ReportBlocksGenerator {
            blocks_report,
            mapper: self
                .enums_mapper
                .iter()
                .map(|v| (v.from.clone(), v.to.clone()))
                .collect(),
        };

        let properties = generator.generate_properties();
        generator.generate_properties_code(&properties, tokens);
        generator.generate_blocks_code_enum(&properties, tokens);
        generator.generate_blocks_code_extra(tokens);
    }
}

pub(crate) fn report_blocks_generate_enum(
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let c = parse_macro_input!(input as ReportBlocksEnum);
    quote! { #c }.into()
}

#[cfg(test)]
mod test {
    use crate::reports::blocks::PropertyType;

    #[test]
    fn test_property_from() {
        assert_eq!(
            PropertyType::from(vec!["true".to_owned(), "false".to_owned()]),
            PropertyType::Boolean,
        );
        assert_eq!(
            PropertyType::from(vec![
                "0".to_owned(),
                "1".to_owned(),
                "2".to_owned(),
                "3".to_owned()
            ]),
            PropertyType::Number(3),
        );
        assert_eq!(
            PropertyType::from(vec![
                "1".to_owned(),
                "2".to_owned(),
                "3".to_owned(),
                "4".to_owned()
            ]),
            PropertyType::Number(3),
        );
        assert_eq!(
            PropertyType::from(vec!["Hello".to_owned(), "World".to_owned()]),
            PropertyType::Enum(vec!["Hello".to_owned(), "World".to_owned()]),
        );
    }
}
