use std::collections::BTreeMap;

use convert_case::Casing as _;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::GeneratedError;

use super::{GeneratedReport, GeneratedReportCode};

#[derive(Deserialize)]
pub struct GeneratedReportRegistriesRegistryEntry {
    pub protocol_id: i32,
}

#[derive(Deserialize)]
pub struct GeneratedReportRegistriesRegistry {
    pub protocol_id: i32,
    pub default: Option<String>,
    pub entries: BTreeMap<String, GeneratedReportRegistriesRegistryEntry>,
}

#[derive(Deserialize)]
pub struct GeneratedReportRegistries(pub BTreeMap<String, GeneratedReportRegistriesRegistry>);

impl GeneratedReport for GeneratedReportRegistries {
    const INPUT_FILE: &'static str = "registries.json";
    fn code(&self) -> Result<Vec<GeneratedReportCode>, GeneratedError> {
        #[derive(Serialize)]
        struct EncodedEntries {
            protocol_id: i32,
            #[serde(skip_serializing_if = "Option::is_none")]
            default: Option<String>,
            entries: BTreeMap<String, i32>,
        }

        fn format_key(key: &str) -> String {
            key.trim_start_matches("minecraft:")
                .replace(":", "_")
                .to_case(convert_case::Case::Pascal)
        }

        fn generated_report(entry: &GeneratedReportRegistriesRegistry, name: &str) -> String {
            format!(
                "pub enum {} {{{}}}\n\nimpl {} {{ pub fn from_value(v: i32) -> Option<Self> {{ match v {{{}, _ => None}} }}\n\npub fn to_value(&self) -> i32 {{ match self {{{}}} }} }}{}",
                name,
                entry
                    .entries
                    .keys()
                    .map(|s| format_key(s))
                    .join(","),
                name,
                entry
                    .entries
                    .iter()
                    .map(|(name, entry)| format!("{} => Some(Self::{})", entry.protocol_id, format_key(name)))
                    .join(","),
                entry
                    .entries
                    .iter()
                    .map(|(name, entry)| format!("Self::{} => {}", format_key(name), entry.protocol_id))
                    .join(","),
                if let Some(default_key) = &entry.default {
                    format!("impl Default for {} {{ fn default() -> Self {{ Self::{} }} }}", name, format_key(default_key))
                } else { "".to_owned() }
            )
        }

        Ok(vec![
            GeneratedReportCode::Json(
                "registries".to_owned(),
                serde_json::to_value(
                    self.0
                        .iter()
                        .map(|(registry_name, registry)| {
                            (
                                registry_name.to_owned(),
                                EncodedEntries {
                                    protocol_id: registry.protocol_id,
                                    default: registry.default.clone(),
                                    entries: registry
                                        .entries
                                        .iter()
                                        .map(|(name, data)| (name.to_owned(), data.protocol_id))
                                        .collect(),
                                },
                            )
                        })
                        .collect::<BTreeMap<String, EncodedEntries>>(),
                )?,
            ),
            GeneratedReportCode::Code(
                "registry".to_owned(),
                vec![
                    generated_report(self.0.get("minecraft:entity_type").unwrap(), "EntityType"),
                    generated_report(
                        self.0.get("minecraft:particle_type").unwrap(),
                        "ParticleType",
                    ),
                ]
                .into_iter()
                .join("\n"),
            ),
        ])
    }
}
