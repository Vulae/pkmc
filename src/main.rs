pub mod packet_reader;
pub mod packet_writer;

use std::{
    collections::VecDeque,
    io::{self, Read, Write},
    net::{TcpListener, TcpStream, ToSocketAddrs},
};

use anyhow::Result;
use base64::Engine as _;
use packet_reader::PacketReader;
use packet_writer::{write_packet, PacketWriter};
use serde::Serialize;

static SERVER_ICON: &[u8] = include_bytes!("../server_icon.png");

#[derive(Serialize)]
struct StatusResponseVersion {
    name: String,
    protocol: u64,
}

#[derive(Serialize)]
struct StatusResponsePlayers {
    max: u64,
    online: u64,
    sample: Vec<StatusResponsePlayerSample>,
}

#[derive(Serialize)]
struct StatusResponsePlayerSample {
    name: String,
    id: String,
}

#[derive(Serialize)]
struct StatusResponseDescription {
    text: String,
}

#[derive(Serialize)]
struct StatusResponse {
    version: StatusResponseVersion,
    players: Option<StatusResponsePlayers>,
    description: Option<StatusResponseDescription>,
    favicon: Option<String>,
    #[serde(rename = "enforcesSecureChat")]
    enforces_secure_chat: bool,
}

#[derive(Debug)]
struct Connection {
    connection_id: u64,
    stream: TcpStream,
    closed: bool,
    bytes: VecDeque<u8>,
    state: i32,
}

impl Connection {
    pub fn new(connection_id: u64, stream: TcpStream) -> Result<Self> {
        stream.set_nonblocking(true)?;
        Ok(Self {
            connection_id,
            stream,
            closed: false,
            bytes: VecDeque::new(),
            state: 0,
        })
    }

    pub fn step(&mut self) -> Result<()> {
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

        // FIXME: This length is probably varint
        if let Some(length) = self.bytes.front() {
            let length = *length as usize;
            if self.bytes.len() > length {
                self.bytes.pop_front();

                let mut packet_data = vec![0u8; length];
                self.bytes.read_exact(&mut packet_data)?;
                println!("{:?}", packet_data);

                let mut reader = PacketReader::new(std::io::Cursor::new(packet_data));
                match reader.read_unsigned_byte()? {
                    0x00 => match self.state {
                        0 => {
                            reader.read_var_int()?; // VERSION
                            reader.read_string()?; // ADDRESS
                            reader.read_unsigned_short()?; // PORT
                            self.state = reader.read_var_int()?; // NEW_STATE
                        }
                        1 => {
                            let mut writer = PacketWriter::new_empty();
                            writer.write_string(&serde_json::to_string(&StatusResponse {
                                version: StatusResponseVersion {
                                    name: "1.21.1".to_string(),
                                    protocol: 767,
                                },
                                players: Some(StatusResponsePlayers {
                                    online: 0,
                                    max: 20,
                                    sample: Vec::new(),
                                }),
                                description: Some(StatusResponseDescription {
                                    text: "Hello, World!".to_string(),
                                }),
                                favicon: Some(format!(
                                    "data:image/png;base64,{}",
                                    base64::prelude::BASE64_STANDARD.encode(SERVER_ICON)
                                )),
                                enforces_secure_chat: false,
                            })?)?;
                            self.stream
                                .write_all(&write_packet(0x00, &writer.into_inner())?)?;
                            self.stream.flush()?;
                        }
                        _ => panic!(),
                    },
                    0x01 => {
                        let mut writer = PacketWriter::new_empty();
                        writer.write_long(reader.read_long()?)?;
                        self.stream
                            .write_all(&write_packet(0x01, &writer.into_inner())?)?;
                        self.stream.flush()?;
                        self.closed = true;
                    }
                    _ => panic!(),
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
struct Server {
    listener: TcpListener,
    connection_counter: u64,
    connections: Vec<Connection>,
}

impl Server {
    pub fn new<S: ToSocketAddrs>(address: S) -> Result<Self> {
        let listener = TcpListener::bind(address)?;
        listener.set_nonblocking(true)?;
        Ok(Self {
            listener,
            connection_counter: 0,
            connections: Vec::new(),
        })
    }

    pub fn step(&mut self) -> Result<()> {
        //println!("{:#?}", std::time::Instant::now());

        if let Ok((stream, _)) = self.listener.accept() {
            self.connections
                .push(Connection::new(self.connection_counter, stream)?);
            println!("Connection {} opened.", self.connection_counter);
            self.connection_counter += 1;
        }

        self.connections
            .iter_mut()
            .map(|connection| connection.step())
            .collect::<Result<Vec<_>, _>>()?;

        self.connections.retain(|connection| {
            if connection.closed {
                println!("Connection {} closed.", connection.connection_id);
            }
            !connection.closed
        });

        Ok(())
    }
}

fn main() -> Result<()> {
    let mut server = Server::new("127.0.0.1:52817")?;

    loop {
        server.step()?;
        std::thread::sleep(std::time::Duration::from_millis(1));
    }

    //Ok(())
}
