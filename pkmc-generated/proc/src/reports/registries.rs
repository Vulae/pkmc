use std::collections::{BTreeMap, HashMap};

use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use serde::Deserialize;
use syn::{
    parse::Parse, parse_macro_input, spanned::Spanned as _, Ident, LitStr, Token, Visibility,
};

use crate::{file_path, fix_identifier};

struct ReportRegistryEnum {
    file: LitStr,
    name: LitStr,
    enum_vis: Visibility,
    enum_name: Ident,
}

impl Parse for ReportRegistryEnum {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            file: input.parse()?,
            name: {
                input.parse::<Token![,]>()?;
                input.parse()?
            },
            enum_vis: {
                input.parse::<Token![,]>()?;
                input.parse()?
            },
            enum_name: input.parse()?,
        })
    }
}

impl ToTokens for ReportRegistryEnum {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        #[derive(Deserialize)]
        struct ReportRegistryEntry {
            protocol_id: i32,
        }

        #[derive(Deserialize)]
        struct ReportRegistry {
            #[allow(unused)]
            protocol_id: i32,
            default: Option<String>,
            entries: BTreeMap<String, ReportRegistryEntry>,
        }

        #[derive(Deserialize)]
        struct ReportRegistries(HashMap<String, ReportRegistry>);

        let file = file_path(&self.file.value());

        let registries: ReportRegistries =
            serde_json::from_reader(std::fs::File::open(&file).expect("Failed to open file"))
                .expect("Failed to parse JSON");

        let registry = registries
            .0
            .get(&self.name.value())
            .expect("Failed to get registry");

        let enum_vis = &self.enum_vis;
        let enum_name = &self.enum_name;

        let keys = registry
            .entries
            .keys()
            .map(|k| fix_identifier(k))
            .map(|k| Ident::new(&k, tokens.span()))
            .collect::<Vec<_>>();
        let keys_raw = registry
            .entries
            .keys()
            .map(|k| LitStr::new(k, tokens.span()))
            .collect::<Vec<_>>();
        let values = registry
            .entries
            .values()
            .map(|v| v.protocol_id)
            .collect::<Vec<_>>();

        tokens.extend(quote! {
            #enum_vis enum #enum_name {
                #(#keys,)*
            }

            impl #enum_name {
                #enum_vis fn to_id(&self) -> i32 {
                    match self {
                        #(Self::#keys => #values,)*
                    }
                }

                #enum_vis fn from_id(id: i32) -> Option<Self> {
                    match id {
                        #(#values => Some(Self::#keys),)*
                        _ => None,
                    }
                }

                #enum_vis fn to_str(&self) -> &str {
                    match self {
                        #(Self::#keys => #keys_raw,)*
                    }
                }

                #enum_vis fn from_str(str: &str) -> Option<Self> {
                    match str {
                        #(#keys_raw => Some(Self::#keys),)*
                        _ => None,
                    }
                }
            }
        });

        if let Some(default) = &registry.default {
            let default = Ident::new(&fix_identifier(default), tokens.span());
            tokens.extend(quote! {
                impl Default for #enum_name {
                    fn default() -> Self {
                        Self::#default
                    }
                }
            });
        }
    }
}

pub(crate) fn report_registry_generate_enum(
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let c = parse_macro_input!(input as ReportRegistryEnum);
    quote! { #c }.into()
}
