pub mod packet;

use std::{
    collections::VecDeque,
    io::{self, Read, Write},
    net::{TcpListener, TcpStream, ToSocketAddrs},
};

use anyhow::Result;
use base64::Engine as _;
use packet::{
    reader::{read_varint_ret_bytes, try_read_varint_ret_bytes, PacketReader},
    writer::PacketWriter,
    Packet,
};

static SERVER_ICON: &[u8] = include_bytes!("../server_icon.png");

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
        Ok(())
    }

    pub fn recieve(&mut self) -> Result<Option<(i32, Box<[u8]>)>> {
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

        Ok(Some((id, data.into_boxed_slice())))
    }

    pub fn step(&mut self) -> Result<()> {
        self.recieve_bytes()?;

        while let Some((id, data)) = self.recieve()? {
            let mut reader = PacketReader::new(std::io::Cursor::new(data.as_ref()));

            match id {
                0 => match self.state {
                    0 => {
                        let handshake = packet::server_list::Handshake::packet_read(&mut reader)?;
                        self.state = handshake.next_state;
                    }
                    1 => self.send(packet::server_list::StatusResponse {
                        version: packet::server_list::StatusResponseVersion {
                            name: "1.21.1".to_string(),
                            protocol: 767,
                        },
                        players: Some(packet::server_list::StatusResponsePlayers {
                            online: 0,
                            max: 20,
                            sample: Vec::new(),
                        }),
                        description: Some(packet::server_list::StatusResponseDescription {
                            text: "Hello, World!".to_string(),
                        }),
                        favicon: Some(format!(
                            "data:image/png;base64,{}",
                            base64::prelude::BASE64_STANDARD.encode(SERVER_ICON)
                        )),
                        enforces_secure_chat: false,
                    })?,
                    _ => panic!(),
                },
                1 => {
                    self.send(packet::server_list::Ping::packet_read(&mut reader)?)?;
                    self.closed = true;
                }
                _ => panic!(),
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
