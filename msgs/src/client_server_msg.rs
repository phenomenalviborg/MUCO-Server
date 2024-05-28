use std::io::{Cursor, Write};

use anyhow::{bail, Context};
use byteorder::{LittleEndian, WriteBytesExt, ReadBytesExt};

use crate::{client_type::ClientType, dequeue::dequeue_msg};

#[derive(Debug, Clone, Copy)]
pub enum Address {
    Client (u32),
    All,
    Other (u32),
}

impl Address {
    pub fn includes(self, connection_id: u32) -> bool {
        match self {
            Address::Client (addressed) => connection_id == addressed,
            Address::All => true,
            Address::Other (sender) => connection_id != sender,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ClientServerMsg<'a> {
    Disconnect,
    BinaryMessageTo (Address, &'a [u8]),
    SetClientType (ClientType),
    Kick (u32),
}

impl<'a> ClientServerMsg<'a> {
    pub fn dequeue_and_decode(input_buffer: &[u8], sender: u32) -> Option<(usize, anyhow::Result<ClientServerMsg>)> {
        let Some((begin, end)) = dequeue_msg(input_buffer) else { return None };
        let msg = Self::decode(&input_buffer[begin..end], sender);
        Some((end, msg))
    }

    pub fn decode(input_buffer: &[u8], sender: u32) -> anyhow::Result<ClientServerMsg> {
        let mut rdr = Cursor::new(&input_buffer);
        let msg_type_index = rdr.read_u32::<LittleEndian>().unwrap();

        let begin = 4;

        let msg = match msg_type_index {
            0 => {
                ClientServerMsg::Disconnect
            }
            1 => {
                let bs = &input_buffer[begin..];
                ClientServerMsg::BinaryMessageTo (Address::All, bs)
            }
            2 => {
                let bs = &input_buffer[begin..];
                ClientServerMsg::BinaryMessageTo (Address::Other (sender), bs)
            }
            3 => {
                let session_id = rdr.read_u32::<LittleEndian>().unwrap();
                let bs = &input_buffer[begin+4..];
                ClientServerMsg::BinaryMessageTo (Address::Client(session_id), bs)
            }
            4 => {
                let client_type_index = rdr.read_u32::<LittleEndian>().unwrap();
                let client_type = ClientType::from_u32(client_type_index).context("unsupported client id")?;
                ClientServerMsg::SetClientType (client_type)
            }
            5 => {
                let session_id = rdr.read_u32::<LittleEndian>().unwrap();
                ClientServerMsg::Kick (session_id)
            }
            type_index => {
                bail!("unsupported msg type: {type_index}");
            }
        };

        Ok(msg)
    }

    pub fn pack(&self, wtr: &mut impl Write) {
        match self {
            ClientServerMsg::Disconnect => {
                wtr.write_u32::<LittleEndian>(4).unwrap();
                wtr.write_u32::<LittleEndian>(0).unwrap();
            }
            ClientServerMsg::BinaryMessageTo(address, bytes) => {
                match address {
                    Address::Client (session_id) => {
                        wtr.write_u32::<LittleEndian>(8 + bytes.len() as u32).unwrap();
                        wtr.write_u32::<LittleEndian>(3).unwrap();
                        wtr.write_u32::<LittleEndian>(*session_id).unwrap();
                        wtr.write_all(bytes).unwrap();
                    }
                    Address::All => {
                        wtr.write_u32::<LittleEndian>(4 + bytes.len() as u32).unwrap();
                        wtr.write_u32::<LittleEndian>(1).unwrap();
                        wtr.write_all(bytes).unwrap();
                    }
                    Address::Other (_) => {
                        wtr.write_u32::<LittleEndian>(4 + bytes.len() as u32).unwrap();
                        wtr.write_u32::<LittleEndian>(2).unwrap();
                        wtr.write_all(bytes).unwrap();
                    }
                }
            }
            ClientServerMsg::SetClientType(client_type) => {
                wtr.write_u32::<LittleEndian>(8).unwrap();
                wtr.write_u32::<LittleEndian>(4).unwrap();
                wtr.write_u32::<LittleEndian>(client_type.as_u32()).unwrap();
            }
            ClientServerMsg::Kick(session_id) => {
                wtr.write_u32::<LittleEndian>(8).unwrap();
                wtr.write_u32::<LittleEndian>(5).unwrap();
                wtr.write_u32::<LittleEndian>(*session_id).unwrap();
            }
        }
    }
}
