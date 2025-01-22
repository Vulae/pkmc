use crate::packet::ConnectionError;

#[derive(Debug, Clone)]
pub struct UncompressedPacketHandler;

impl UncompressedPacketHandler {
    pub fn write(&self, raw: &[u8]) -> Result<Box<[u8]>, ConnectionError> {
        Ok(raw.to_owned().into_boxed_slice())
    }

    pub fn read(&self, buf: &[u8]) -> Result<Box<[u8]>, ConnectionError> {
        Ok(buf.to_owned().into_boxed_slice())
    }
}
