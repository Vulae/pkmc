use std::collections::HashMap;

use itertools::Itertools;
use serde::Deserialize;

use crate::GeneratedError;

use super::{GeneratedReport, GeneratedReportCode};

#[derive(Deserialize)]
pub struct GeneratedReportPacketsStatePacket {
    pub protocol_id: i32,
}

#[derive(Deserialize)]
pub struct GeneratedReportPacketsState {
    #[serde(default)]
    pub clientbound: HashMap<String, GeneratedReportPacketsStatePacket>,
    #[serde(default)]
    pub serverbound: HashMap<String, GeneratedReportPacketsStatePacket>,
}

#[derive(Deserialize)]
pub struct GeneratedReportPackets(pub HashMap<String, GeneratedReportPacketsState>);

impl GeneratedReport for GeneratedReportPackets {
    const INPUT_FILE: &'static str = "packets.json";
    fn code(&self) -> Result<Vec<GeneratedReportCode>, GeneratedError> {
        Ok(vec![GeneratedReportCode::Code(
            "packet".to_owned(),
            self.0
                .iter()
                .sorted_by(|(state_1, _), (state_2, _)| state_1.cmp(state_2))
                .map(|(state, packets)| {
                    format!(
                        "pub mod {} {{\n{}\n}}",
                        state,
                        packets
                            .clientbound
                            .iter()
                            .map(|(packet_name, packet)| (
                                "clientbound".to_owned(),
                                packet_name,
                                packet
                            ))
                            .sorted_by(|(_, packet_name_1, _), (_, packet_name_2, _)| packet_name_1
                                .cmp(packet_name_2))
                            .chain(
                                packets
                                    .serverbound
                                    .iter()
                                    .map(|(packet_name, packet)| (
                                        "serverbound".to_owned(),
                                        packet_name,
                                        packet
                                    ))
                                    .sorted_by(|(_, packet_name_1, _), (_, packet_name_2, _)| {
                                        packet_name_1.cmp(packet_name_2)
                                    })
                            )
                            .map(|(boundedness, packet_name, packet)| {
                                format!(
                                    "pub const {}_{}: i32 = {};",
                                    boundedness.to_uppercase(),
                                    packet_name.to_uppercase().replace(":", "_"),
                                    packet.protocol_id
                                )
                            })
                            .collect::<Vec<String>>()
                            .join("\n"),
                    )
                })
                .collect::<Vec<String>>()
                .join("\n\n"),
        )])
    }
}
