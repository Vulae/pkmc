use std::io::Write;

use pkmc_util::{
    Vec3,
    connection::{ClientboundPacket, ConnectionError, PacketEncoder as _},
};

use crate::particle::{self, Particle};

#[derive(Debug)]
pub struct LevelParticles {
    pub long_distance: bool,
    pub always_visible: bool,
    pub position: Vec3<f64>,
    pub offset: Vec3<f32>,
    pub max_speed: f32,
    pub particle_count: i32,
    pub particle: Particle,
}

impl ClientboundPacket for LevelParticles {
    const CLIENTBOUND_ID: i32 = pkmc_generated::packet::play::CLIENTBOUND_LEVEL_PARTICLES;

    fn packet_write(&self, mut writer: impl Write) -> Result<(), ConnectionError> {
        writer.encode(self.long_distance)?;
        writer.encode(self.always_visible)?;
        writer.write_all(&self.position.x.to_be_bytes())?;
        writer.write_all(&self.position.y.to_be_bytes())?;
        writer.write_all(&self.position.z.to_be_bytes())?;
        writer.write_all(&self.offset.x.to_be_bytes())?;
        writer.write_all(&self.offset.y.to_be_bytes())?;
        writer.write_all(&self.offset.z.to_be_bytes())?;
        writer.write_all(&self.max_speed.to_be_bytes())?;
        writer.write_all(&self.particle_count.to_be_bytes())?;
        writer.encode(self.particle.r#type().to_id())?;
        match &self.particle {
            Particle::Block(block) => {
                writer.encode(block.into_id())?;
            }
            Particle::BlockMarker(block) => {
                writer.encode(block.into_id())?;
            }
            Particle::Dust { color, scale } => {
                writer.write_all(&color.to_argb8888(0).to_be_bytes())?;
                writer.write_all(&scale.to_be_bytes())?;
            }
            Particle::DustColorTransition { from, to, scale } => {
                writer.write_all(&from.to_argb8888(0).to_be_bytes())?;
                writer.write_all(&to.to_argb8888(0).to_be_bytes())?;
                writer.write_all(&scale.to_be_bytes())?;
            }
            Particle::EntityEffect { color, alpha } => {
                writer.write_all(&color.to_argb8888(*alpha).to_be_bytes())?;
            }
            Particle::FallingDust(block) => {
                writer.encode(block.into_id())?;
            }
            Particle::SculkCharge { roll } => {
                writer.write_all(&roll.to_be_bytes())?;
            }
            Particle::Item => unimplemented!(),
            Particle::Vibration { source, ticks } => {
                match source {
                    particle::VibrationSource::Block(position) => {
                        writer.encode(0)?;
                        writer.encode(position)?;
                    }
                    particle::VibrationSource::Entity { id, eye_height } => {
                        writer.encode(1)?;
                        writer.encode(*id)?;
                        writer.write_all(&eye_height.to_be_bytes())?;
                    }
                }
                writer.encode(*ticks)?;
            }
            Particle::Trail {
                position,
                color,
                duration,
            } => {
                writer.write_all(&position.x.to_be_bytes())?;
                writer.write_all(&position.y.to_be_bytes())?;
                writer.write_all(&position.z.to_be_bytes())?;
                writer.write_all(&color.to_argb8888(0).to_be_bytes())?;
                writer.encode(*duration)?;
            }
            Particle::Shriek { delay } => {
                writer.encode(*delay)?;
            }
            Particle::DustPillar(block) => {
                writer.encode(block.into_id())?;
            }
            Particle::BlockCrumble(block) => {
                writer.encode(block.into_id())?;
            }
            _ => {}
        }
        Ok(())
    }
}
