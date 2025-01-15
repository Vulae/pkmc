use std::collections::{BTreeMap, HashMap};

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
    pub entries: HashMap<String, GeneratedReportRegistriesRegistryEntry>,
}

#[derive(Deserialize)]
pub struct GeneratedReportRegistries(pub HashMap<String, GeneratedReportRegistriesRegistry>);

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

        Ok(vec![GeneratedReportCode::Json(
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
        )])
    }
}
