use crate::{client_login::ClientLogin, connection::Connection, packet, uuid::UUID};
use anyhow::{anyhow, Result};

#[derive(Debug)]
pub struct Client {
    connection: Connection,
    pub name: String,
    pub uuid: UUID,
}

impl Client {
    pub fn initialize(login_player: ClientLogin) -> Result<Self> {
        let Some((name, uuid)) = &login_player.player else {
            return Err(anyhow!("Login player incomplete"));
        };
        let name = name.clone();
        let uuid = *uuid;

        println!("Player initialize {}", name);

        let mut client = Client {
            connection: login_player.into_connection(),
            name,
            uuid,
        };

        client
            .connection
            .send(packet::play::SynchronizePlayerPosition {
                relative: false,
                x: Some(0.0),
                y: Some(0.0),
                z: Some(0.0),
                yaw: Some(0.0),
                pitch: Some(0.0),
                teleport_id: 0,
            })?;

        client.connection.send(packet::play::GameEvent {
            event: 13,
            value: 0.0,
        })?;

        Ok(client)
    }
}
