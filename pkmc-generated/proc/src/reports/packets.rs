use std::collections::BTreeMap;

use convert_case::Casing;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use serde::Deserialize;
use syn::{parse::Parse, parse_macro_input, spanned::Spanned as _, Ident, LitStr};

use crate::{file_path, fix_identifier};

struct ReportPacketsConstants {
    file: LitStr,
}

impl Parse for ReportPacketsConstants {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            file: input.parse()?,
        })
    }
}

impl ToTokens for ReportPacketsConstants {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        #[derive(Deserialize)]
        struct ReportPacket {
            protocol_id: i32,
        }

        #[derive(Deserialize)]
        struct ReportPacketGroup {
            #[serde(default)]
            clientbound: BTreeMap<String, ReportPacket>,
            #[serde(default)]
            serverbound: BTreeMap<String, ReportPacket>,
        }

        #[derive(Deserialize)]
        struct ReportPackets(BTreeMap<String, ReportPacketGroup>);

        let file = file_path(&self.file.value());

        let registries: ReportPackets =
            serde_json::from_reader(std::fs::File::open(&file).expect("Failed to open file"))
                .expect("Failed to parse JSON");

        registries.0.iter().for_each(|(group_name, group)| {
            let mut tokens_group = TokenStream::new();

            [
                ("clientbound", &group.clientbound),
                ("serverbound", &group.serverbound),
            ]
            .into_iter()
            .for_each(|(boundedness, packets)| {
                let names = packets
                    .keys()
                    .map(|k| {
                        format!("{}_{}", boundedness, fix_identifier(k))
                            .to_case(convert_case::Case::Constant)
                    })
                    .map(|k| Ident::new(&k, tokens.span()))
                    .collect::<Vec<_>>();
                let values = packets.values().map(|v| v.protocol_id).collect::<Vec<_>>();
                tokens_group.extend(quote! {
                    #(pub const #names: i32 = #values;)*
                });
            });

            let group_name = Ident::new(group_name, tokens.span());
            tokens.extend(quote! {
                pub mod #group_name {
                    #tokens_group
                }
            });
        });
    }
}

pub(crate) fn report_packets_generate_consts(
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let c = parse_macro_input!(input as ReportPacketsConstants);
    quote! { #c }.into()
}
