use anyhow::Result;
use std::{
    collections::VecDeque,
    io::{self, Read as _, Write as _},
    net::TcpStream,
};

use crate::packet::{
    reader::{read_varint_ret_bytes, try_read_varint_ret_bytes},
    writer::PacketWriter,
    Packet,
};

#[derive(Debug)]
pub struct Connection {
    stream: TcpStream,
    closed: bool,
    bytes: VecDeque<u8>,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Result<Self> {
        stream.set_nonblocking(true)?;
        Ok(Self {
            stream,
            closed: false,
            bytes: VecDeque::new(),
        })
    }

    fn recieve_bytes(&mut self) -> Result<()> {
        let mut buf = [0u8; 1024];
        loop {
            match self.stream.read(&mut buf) {
                Ok(0) => {
                    self.closed = true;
                    return Ok(());
                }
                Ok(n) => {
                    self.bytes.extend(&buf[..n]);
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    break;
                }
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
    }

    pub fn send(&mut self, packet: impl Packet) -> Result<()> {
        // TODO: Rewrite please, I'm sorry for this, this is pretty dumb.
        let mut writer_data = PacketWriter::new_empty();
        packet.packet_write(&mut writer_data)?;

        let mut writer_id = PacketWriter::new_empty();
        writer_id.write_var_int(packet.id())?;

        let contents = writer_id
            .into_inner()
            .into_iter()
            .chain(writer_data.into_inner())
            .collect::<Vec<_>>()
            .into_boxed_slice();

        PacketWriter::new(&mut self.stream).write_var_int(contents.len() as i32)?;
        self.stream.write_all(&contents)?;
        self.stream.flush()?;

        println!("SENT {}", packet.id());

        Ok(())
    }

    pub fn recieve(&mut self) -> Result<Option<(i32, Box<[u8]>)>> {
        self.recieve_bytes()?;

        // TODO: Rewrite please, I'm sorry for this as well, This is way more dumb.
        let front = [
            self.bytes.front(),
            self.bytes.get(1),
            self.bytes.get(2),
            self.bytes.get(3),
            self.bytes.get(4),
        ]
        .into_iter()
        .filter_map(|v| v.cloned())
        .collect::<Vec<_>>();
        let Some((length_bytes, length)) = try_read_varint_ret_bytes(&front)? else {
            return Ok(None);
        };

        if self.bytes.len() < length_bytes + length as usize {
            return Ok(None);
        }

        (0..length_bytes).for_each(|_| {
            self.bytes.pop_front();
        });

        let mut data = vec![0u8; length as usize];
        self.bytes.read_exact(&mut data)?;

        let (id_length, id) = read_varint_ret_bytes(std::io::Cursor::new(&data))?;
        // ;-;
        (0..id_length).for_each(|_| {
            data.remove(0);
        });

        println!("RECIEVE {}", id);

        Ok(Some((id, data.into_boxed_slice())))
    }
}
