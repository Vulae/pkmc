use std::io::{Read, Write};

use pkmc_util::{
    connection::{
        ClientboundPacket, ConnectionError, PacketDecoder as _, PacketEncoder as _,
        ServerboundPacket,
    },
    FixedBitSet, ReadExt as _,
};

use crate::text_component::TextComponent;

#[derive(Debug)]
pub struct SystemChat {
    pub content: TextComponent,
    pub overlay: bool,
}

impl ClientboundPacket for SystemChat {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_SYSTEM_CHAT;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(&self.content.to_nbt())?;
        writer.encode(self.overlay)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct SetActionBarText(pub TextComponent);

impl ClientboundPacket for SetActionBarText {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_SET_ACTION_BAR_TEXT;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(&self.0.to_nbt())?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct ChatMessage {
    pub message: String,
    pub timestamp: i64,
    pub salt: i64,
    pub signature: Option<[u8; 256]>,
    pub message_count: i32,
    pub acknowledged: FixedBitSet<20>,
}

impl ServerboundPacket for ChatMessage {
    const SERVERBOUND_ID: i32 = pkmc_generated::packet::play::SERVERBOUND_CHAT;

    fn packet_read(mut reader: impl Read) -> Result<Self, ConnectionError>
    where
        Self: Sized,
    {
        Ok(Self {
            message: reader.decode()?,
            timestamp: i64::from_be_bytes(reader.read_const()?),
            salt: i64::from_be_bytes(reader.read_const()?),
            signature: reader
                .decode::<bool>()?
                .then(|| reader.read_const())
                .transpose()?,
            message_count: reader.decode()?,
            acknowledged: reader.decode()?,
        })
    }
}

#[derive(Debug)]
pub struct DisguisedChatMessage {
    pub message: TextComponent,
    // TODO: minecraft:chat_type registry generated code enum
    pub chat_type: i32,
    pub sender_name: TextComponent,
    pub target_name: Option<TextComponent>,
}

impl ClientboundPacket for DisguisedChatMessage {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_DISGUISED_CHAT;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(&self.message.to_nbt())?;
        writer.encode(self.chat_type + 1)?;
        writer.encode(&self.sender_name.to_nbt())?;
        if let Some(target_name) = &self.target_name {
            writer.encode(true)?;
            writer.encode(&target_name.to_nbt())?;
        } else {
            writer.encode(false)?;
        }
        Ok(())
    }
}
