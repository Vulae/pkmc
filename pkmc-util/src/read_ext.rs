use std::io::Read;

pub trait ReadExt {
    fn read_const<const N: usize>(&mut self) -> std::io::Result<[u8; N]>;
    fn read_var(&mut self, size: usize) -> std::io::Result<Box<[u8]>>;
    fn read_all(&mut self) -> std::io::Result<Box<[u8]>>;
}

impl<T: Read> ReadExt for T {
    fn read_const<const N: usize>(&mut self) -> std::io::Result<[u8; N]> {
        let mut buf = [0u8; N];
        self.read_exact(&mut buf)?;
        Ok(buf)
    }

    fn read_var(&mut self, size: usize) -> std::io::Result<Box<[u8]>> {
        let mut buf = vec![0u8; size].into_boxed_slice();
        self.read_exact(&mut buf)?;
        Ok(buf)
    }

    fn read_all(&mut self) -> std::io::Result<Box<[u8]>> {
        let mut data = Vec::new();
        self.read_to_end(&mut data)?;
        Ok(data.into_boxed_slice())
    }
}
