use crate::packet::ConnectionError;

#[derive(Debug, Clone)]
pub struct UncompressedPacketHandler;

impl UncompressedPacketHandler {
    pub(super) fn write(&self, raw: &[u8]) -> Result<Box<[u8]>, ConnectionError> {
        Ok(raw.to_owned().into_boxed_slice())
    }

    pub(super) fn read(&self, buf: &[u8]) -> Result<Box<[u8]>, ConnectionError> {
        Ok(buf.to_owned().into_boxed_slice())
    }
}
