use std::io::{Cursor, Write};

use anyhow::{bail, Context};
use byteorder::{LittleEndian, WriteBytesExt, ReadBytesExt};

use crate::{client_type::ClientType, dequeue::dequeue_msg};

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
        let Some((begin, end)) = dequeue_msg(input_buffer) else { return None };
        let msg = Self::decode(&input_buffer[begin..end]);
        input_buffer.drain(..end);
        Some(msg)
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

    pub fn pack(&self, wtr: &mut impl Write) {
        match self {
            ClientServerMsg::Disconnect => {
                wtr.write_u32::<LittleEndian>(4).unwrap();
                wtr.write_u32::<LittleEndian>(0).unwrap();
            }
            ClientServerMsg::BroadcastBytesAll(bytes) => {
                wtr.write_u32::<LittleEndian>(4 + bytes.len() as u32).unwrap();
                wtr.write_u32::<LittleEndian>(2).unwrap();
                wtr.write_all(bytes).unwrap();
            }
            ClientServerMsg::BroadcastBytesOther(bytes) => {
                wtr.write_u32::<LittleEndian>(4 + bytes.len() as u32).unwrap();
                wtr.write_u32::<LittleEndian>(3).unwrap();
                wtr.write_all(bytes).unwrap();
            }
            ClientServerMsg::BinaryMessageTo(address, bytes) => {
                wtr.write_u32::<LittleEndian>(8 + bytes.len() as u32).unwrap();
                wtr.write_u32::<LittleEndian>(6).unwrap();
                wtr.write_u32::<LittleEndian>(*address as u32).unwrap();
                wtr.write_all(bytes).unwrap();
            }
            ClientServerMsg::SetClientType(client_type) => {
                wtr.write_u32::<LittleEndian>(8).unwrap();
                wtr.write_u32::<LittleEndian>(7).unwrap();
                wtr.write_u32::<LittleEndian>(client_type.as_u32()).unwrap();
            }
        }
    }
}
