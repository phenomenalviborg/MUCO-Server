use std::io::{Cursor, Write};

use anyhow::bail;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::dequeue::dequeue_msg;

#[derive(Debug, Clone)]
pub enum ServerClientMsg {
    AssignClientId (usize),
    ClientConnected (usize),
    ClientDisconnected (usize),
    BroadcastBytes (usize, Vec<u8>),
    BinaryMessageFrom (usize, Vec<u8>),
}

impl ServerClientMsg {
    pub fn dequeue_and_decode(input_buffer: &mut Vec<u8>) -> Option<anyhow::Result<ServerClientMsg>> {
        let Some((begin, end)) = dequeue_msg(input_buffer) else { return None };
        let msg = Self::decode(&input_buffer[begin..end]);
        input_buffer.drain(..end);
        Some(msg)
    }

    pub fn decode(input_buffer: &[u8]) -> anyhow::Result<ServerClientMsg> {
        let mut rdr = Cursor::new(&input_buffer);
        let msg_type_index = rdr.read_u32::<LittleEndian>().unwrap();

        let begin = 4;

        let msg = match msg_type_index {
            0 => {
                let client_id = rdr.read_u32::<LittleEndian>().unwrap() as usize;
                ServerClientMsg::AssignClientId(client_id)
            }
            1 => {
                let client_id = rdr.read_u32::<LittleEndian>().unwrap() as usize;
                ServerClientMsg::ClientConnected(client_id)
            }
            2 => {
                let client_id = rdr.read_u32::<LittleEndian>().unwrap() as usize;
                ServerClientMsg::ClientDisconnected(client_id)
            }
            3 => {
                let sender = rdr.read_u32::<LittleEndian>().unwrap() as usize;
                let bs = input_buffer[begin+4..].to_vec();
                ServerClientMsg::BroadcastBytes (sender, bs)
            }
            4 => {
                let sender = rdr.read_u32::<LittleEndian>().unwrap() as usize;
                let bs = input_buffer[begin+4..].to_vec();
                ServerClientMsg::BinaryMessageFrom (sender, bs)
            }
            type_index => {
                bail!("unsupported msg type: {type_index}");
            }
        };

        Ok(msg)
    }

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
                wtr.write_u32::<LittleEndian>(3).unwrap();
                wtr.write_u32::<LittleEndian>(*sender as u32).unwrap();
                wtr.write_all(bytes).unwrap();
            }
            ServerClientMsg::BinaryMessageFrom (sender, bytes) => {
                wtr.write_u32::<LittleEndian>(8 + bytes.len() as u32).unwrap();
                wtr.write_u32::<LittleEndian>(4).unwrap();
                wtr.write_u32::<LittleEndian>(*sender as u32).unwrap();
                wtr.write_all(bytes).unwrap();
            }
        }
    }
}
