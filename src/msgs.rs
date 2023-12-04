use std::io::{Cursor, Write};

use anyhow::{bail, Context};
use byteorder::{WriteBytesExt, LittleEndian, ReadBytesExt};

#[derive(Debug, Clone, Copy)]
pub enum ClientType {
    Player,
    Manager,
}

impl ClientType {
    pub fn from_u32(index: u32) -> Option<ClientType> {
        match index {
            0 => Some(ClientType::Player),
            1 => Some(ClientType::Manager),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ClientServerMsg {
    Disconnect,
    //BroadcastChatMessage
    BroadcastBytesAll (Vec<u8>),
    BroadcastBytesOther (Vec<u8>),
    //StoreData
    //RetrieveData
    BinaryMessageTo (usize, Vec<u8>),
    SetClientType (ClientType),
}

impl ClientServerMsg {
    pub fn dequeue_and_decode(input_buffer: &mut Vec<u8>) -> Option<anyhow::Result<ClientServerMsg>> {
        let Some((begin, end)) = Self::dequeue(input_buffer) else { return None };
        let msg = Self::decode(&input_buffer[begin..end]);
        input_buffer.drain(..end);
        Some(msg)
    }

    pub fn dequeue(input_buffer: &mut Vec<u8>) -> Option<(usize, usize)> {
        if input_buffer.len() < 4 {
            return None
        }

        let mut rdr = Cursor::new(&input_buffer);

        let msg_ln = rdr.read_u32::<LittleEndian>().unwrap() as usize;

        if msg_ln > 2000 {
            println!("long message: {msg_ln}");
        }

        let end = msg_ln + 4;

        if input_buffer.len() < end {
            return None
        }

        Some((4, end))
    }

    pub fn decode(input_buffer: &[u8]) -> anyhow::Result<ClientServerMsg> {
        let mut rdr = Cursor::new(&input_buffer);
        let msg_type_index = rdr.read_u32::<LittleEndian>().unwrap();

        let begin = 4;

        let msg = match msg_type_index {
            0 => {
                ClientServerMsg::Disconnect
            }
            2 => {
                let bs = input_buffer[begin..].to_vec();
                ClientServerMsg::BroadcastBytesAll (bs)
            }
            3 => {
                let bs = input_buffer[begin..].to_vec();
                ClientServerMsg::BroadcastBytesOther (bs)
            }
            6 => {
                let address = rdr.read_u32::<LittleEndian>().unwrap() as usize;
                let bs = input_buffer[begin+4..].to_vec();
                ClientServerMsg::BinaryMessageTo (address, bs)
            }
            7 => {
                let client_type_index = rdr.read_u32::<LittleEndian>().unwrap();
                let client_type = ClientType::from_u32(client_type_index).context("unsupported client id")?;
                ClientServerMsg::SetClientType (client_type)
            }
            type_index => {
                bail!("unsupported msg type: {type_index}");
            }
        };

        Ok(msg)
    }
}

#[derive(Debug, Clone)]
pub enum ServerClientMsg {
    AssignClientId (usize),
    ClientConnected (usize),
    ClientDisconnected (usize),
    //BroadcastChatMessage
    BroadcastBytes (usize, Vec<u8>),
    //Data
    BinaryMessageFrom (usize, Vec<u8>),
}

impl ServerClientMsg {
    pub fn pack(&self, wtr: &mut impl Write) {
        match self {
            ServerClientMsg::AssignClientId (id) => {
                wtr.write_u32::<LittleEndian>(8).unwrap();
                wtr.write_u32::<LittleEndian>(0).unwrap();
                wtr.write_u32::<LittleEndian>(*id as u32).unwrap();
            }
            ServerClientMsg::ClientConnected (id) => {
                wtr.write_u32::<LittleEndian>(8).unwrap();
                wtr.write_u32::<LittleEndian>(1).unwrap();
                wtr.write_u32::<LittleEndian>(*id as u32).unwrap();
            }
            ServerClientMsg::ClientDisconnected (id) => {
                wtr.write_u32::<LittleEndian>(8).unwrap();
                wtr.write_u32::<LittleEndian>(2).unwrap();
                wtr.write_u32::<LittleEndian>(*id as u32).unwrap();
            }
            ServerClientMsg::BroadcastBytes (sender, bytes) => {
                wtr.write_u32::<LittleEndian>(8 + bytes.len() as u32).unwrap();
                wtr.write_u32::<LittleEndian>(4).unwrap();
                wtr.write_u32::<LittleEndian>(*sender as u32).unwrap();
                wtr.write_all(bytes).unwrap();
            }
            ServerClientMsg::BinaryMessageFrom (sender, bytes) => {
                wtr.write_u32::<LittleEndian>(8 + bytes.len() as u32).unwrap();
                wtr.write_u32::<LittleEndian>(6).unwrap();
                wtr.write_u32::<LittleEndian>(*sender as u32).unwrap();
                wtr.write_all(bytes).unwrap();
            }
        }
    }
}
