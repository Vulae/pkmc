use std::collections::{BTreeMap, HashMap};

use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::GeneratedError;

use super::{GeneratedReport, GeneratedReportCode};

fn is_false(bool: &bool) -> bool {
    !bool
}

#[derive(Deserialize, Serialize)]
pub struct GeneratedReportBlocksBlockState {
    #[serde(skip_serializing_if = "is_false")]
    #[serde(default)]
    pub default: bool,
    pub id: i32,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    #[serde(default)]
    pub properties: BTreeMap<String, String>,
}

#[derive(Deserialize, Serialize)]
pub struct GeneratedReportBlocksBlockDefinition {
    pub r#type: String,
}

#[derive(Deserialize, Serialize)]
pub struct GeneratedReportBlocksBlock {
    pub definition: GeneratedReportBlocksBlockDefinition,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    #[serde(default)]
    pub properties: BTreeMap<String, Vec<String>>,
    pub states: Vec<GeneratedReportBlocksBlockState>,
}

#[derive(Deserialize)]
pub struct GeneratedReportBlocks(pub HashMap<String, GeneratedReportBlocksBlock>);

impl GeneratedReport for GeneratedReportBlocks {
    const INPUT_FILE: &'static str = "blocks.json";
    fn code(&self) -> Result<Vec<GeneratedReportCode>, GeneratedError> {
        Ok(vec![
            GeneratedReportCode::Json("block".to_owned(), serde_json::to_value(&self.0)?),
            GeneratedReportCode::Code(
                "block".to_owned(),
                format!(
                    "#[inline(always)]\npub const fn is_air(id: i32) -> bool {{\nmatches!(id, {})\n}}",
                    vec!["minecraft:air", "minecraft:cave_air", "minecraft:void_air"].into_iter().map(|air_name| {
                        self.0.get(air_name)
                            .unwrap()
                            .states.iter()
                            .find(|state| state.default).unwrap()
                            .id
                    }).join("|"),
                ),
            ),
        ])

        //#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        //enum States {
        //    Boolean,
        //    Enum(Vec<String>),
        //}
        //
        //impl States {
        //    fn from_enum(states: &[String]) -> Self {
        //        if states == ["true", "false"] {
        //            return Self::Boolean;
        //        }
        //        Self::Enum(states.to_vec())
        //    }
        //}
        //
        //let mut states_definitions: BTreeMap<String, BTreeSet<States>> = BTreeMap::new();
        //
        //self.0
        //    .values()
        //    .flat_map(|block| block.properties.clone())
        //    .map(|(name, properties)| (name, States::from_enum(&properties)))
        //    .for_each(|(name, values)| {
        //        let existing_enums = states_definitions.entry(name.clone()).or_default();
        //        existing_enums.insert(values);
        //    });
        //
        //// (property_name, enum_name, states)
        //let mut states_enums: Vec<(String, String, Vec<String>)> = Vec::new();
        //
        //states_definitions.iter().for_each(|(name, defs)| {
        //    let defs = defs
        //        .iter()
        //        .flat_map(|def| match def {
        //            States::Boolean => None,
        //            States::Enum(values) => Some(values),
        //        })
        //        .collect::<Vec<_>>();
        //    if defs.len() == 1 {
        //        states_enums.push((name.to_owned(), name.to_case(Case::Pascal), defs[0].clone()));
        //    } else {
        //        defs.iter().enumerate().for_each(|(i, def)| {
        //            states_enums.push((
        //                name.to_owned(),
        //                format!("{}{}", name, i).to_case(Case::Pascal),
        //                def.to_vec(),
        //            ));
        //        });
        //    }
        //});
        //
        //fn block_enum_identifier(identifier: &str) -> String {
        //    identifier
        //        .trim_start_matches("minecraft:")
        //        .replace(":", "_")
        //        .to_case(Case::Pascal)
        //}
        //
        //fn safe_rust_ident(ident: &str) -> &str {
        //    match ident {
        //        "type" => "r#type",
        //        _ => ident,
        //    }
        //}
        //
        //let blocks_enum_inner = self
        //    .0
        //    .iter()
        //    .sorted_by(|(k1, _), (k2, _)| k1.cmp(k2))
        //    .map(|(identifier, block)| {
        //        if block.properties.is_empty() {
        //            block_enum_identifier(identifier)
        //        } else {
        //            format!(
        //                "{} {{\n{}\n}}",
        //                block_enum_identifier(identifier),
        //                block
        //                    .properties
        //                    .iter()
        //                    .map(|(property, states)| {
        //                        let states = States::from_enum(states);
        //                        match states {
        //                            States::Boolean => {
        //                                format!("{}: bool,", safe_rust_ident(property))
        //                            }
        //                            States::Enum(values) => {
        //                                let enum_name = &states_enums
        //                                    .iter()
        //                                    .find(|(property_name, _enum_name, enum_values)| {
        //                                        property_name == property && values == *enum_values
        //                                    })
        //                                    .unwrap()
        //                                    .1;
        //                                format!("{}: {},", safe_rust_ident(property), enum_name)
        //                            }
        //                        }
        //                    })
        //                    .join("\n")
        //            )
        //        }
        //    })
        //    .join(",\n");
        //
        //let blocks_idable_to_inner = self
        //    .0
        //    .iter()
        //    .sorted_by(|(k1, _), (k2, _)| k1.cmp(k2))
        //    .map(|(identifier, block)| {
        //        if block.properties.is_empty() {
        //            format!(
        //                "Self::{} => {},",
        //                block_enum_identifier(identifier),
        //                block.states.first().unwrap().id
        //            )
        //        } else {
        //            block
        //                .states
        //                .iter()
        //                .map(|state| {
        //                    format!(
        //                        "Self::{} {{{}}} => {},",
        //                        block_enum_identifier(identifier),
        //                        state.properties.iter().for_each(|name, value| {}),
        //                        state.id,
        //                    )
        //                })
        //                .join("\n")
        //            //// TODO: Block states, I'm too lazy right now to figure this out.
        //            //"".to_owned()
        //        }
        //    })
        //    .filter(|str| !str.trim().is_empty())
        //    .join("\n");
        //
        //#[allow(clippy::format_in_format_args)]
        //Ok(format!(
        //    "use crate::{{generated_util_create_basic_enum, IdAble}};\n\n{}\n\n{}",
        //    states_definitions
        //        .iter()
        //        .map(|(name, definitions)| {
        //            definitions
        //                .iter()
        //                .flat_map(|definition| match definition {
        //                    States::Boolean => None,
        //                    States::Enum(values) => Some(values),
        //                })
        //                .enumerate()
        //                .map(|(i, definition)| {
        //                    format!(
        //                        "generated_util_create_basic_enum!(pub {}; {});",
        //                        format!(
        //                            "{}{}",
        //                            name,
        //                            if definitions.len() == 1 {
        //                                "".to_owned()
        //                            } else {
        //                                format!("_{}", i)
        //                            },
        //                        )
        //                        .to_case(Case::Pascal),
        //                        definition
        //                            .iter()
        //                            .map(|def_name| {
        //                                if def_name.chars().nth(0).unwrap().is_ascii_alphabetic() {
        //                                    def_name.to_owned()
        //                                } else {
        //                                    format!("N{}", def_name)
        //                                }
        //                                .to_case(Case::Pascal)
        //                            })
        //                            .join(", "),
        //                    )
        //                })
        //                .join("\n")
        //        })
        //        .join(""),
        //    format!(
        //        "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\npub enum Block {{\n{}\n}}\n\nimpl IdAble for Block {{\nfn from_id(id: u32) -> Option<Self> {{ unimplemented!() }}\nfn to_id(&self) -> u32 {{ match self {{\n{}\n_ => unimplemented!(),\n}} }}\n}}\n\nimpl Default for Block {{\nfn default() -> Self {{ Self::Air }}\n}}",
        //        blocks_enum_inner,
        //        blocks_idable_to_inner,
        //    ),
        //))
    }
}
