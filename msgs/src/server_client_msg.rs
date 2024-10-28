use std::io::{Cursor, Read, Write};

use anyhow::bail;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::{dequeue::dequeue_msg, model::Model};

#[derive(Debug, Clone)]
pub enum ServerClientMsg<'a> {
    Hello {
        session_id: u16,
        model: Model,
    },
    ClientConnected (u16),
    ClientDisconnected (u16),
    InterClient (u16, &'a[u8]),
    DataNotify {
        room: u8,
        component_type: u8,
        creator_id: u16,
        index: u16,
        data: &'a[u8]
    },
    DataOwner {
        room: u8,
        creator_id: u16,
        index: u16,
        owner_id: u16,
    }
}

impl<'a> ServerClientMsg<'a> {
    pub fn dequeue_and_decode_(input_buffer: &mut Vec<u8>) -> Option<(usize, anyhow::Result<ServerClientMsg>)> {
        let Some((begin, end)) = dequeue_msg(input_buffer) else { return None };
        let msg = Self::decode(&input_buffer[begin..end]);
        Some((end, msg))
    }

    pub fn decode(input_buffer: &[u8]) -> anyhow::Result<ServerClientMsg> {
        let mut rdr = Cursor::new(&input_buffer);
        let msg_type_index = rdr.read_u32::<LittleEndian>().unwrap();

        let begin = 4;

        let msg = match msg_type_index {
            0 => {
                let session_id = rdr.read_u16::<LittleEndian>().unwrap();
                let mut model = Model::new();
                let fact_count = rdr.read_u32::<LittleEndian>().unwrap();
                for _ in 0..fact_count {
                    let room = rdr.read_u8().unwrap();
                    let component_type = rdr.read_u8().unwrap();
                    let creator_id = rdr.read_u16::<LittleEndian>().unwrap();
                    let index = rdr.read_u16::<LittleEndian>().unwrap();
                    let len = rdr.read_u32::<LittleEndian>().unwrap();
                    let mut buffer = vec![0u8; len as usize].into_boxed_slice();
                    rdr.read_exact(&mut buffer).unwrap();
                    model.facts.insert((room, component_type, creator_id, index), buffer);
                }
                ServerClientMsg::Hello {
                    session_id,
                    model,
                }
            }
            1 => {
                let session_id = rdr.read_u16::<LittleEndian>().unwrap();
                ServerClientMsg::ClientConnected (session_id)
            }
            2 => {
                let session_id = rdr.read_u16::<LittleEndian>().unwrap();
                ServerClientMsg::ClientDisconnected (session_id)
            }
            3 => {
                let sender = rdr.read_u16::<LittleEndian>().unwrap();
                let bs = &input_buffer[begin+4..];
                ServerClientMsg::InterClient (sender, bs)
            }
            4 => {
                let room = rdr.read_u8().unwrap();
                let component_type = rdr.read_u8().unwrap();
                let creator_id = rdr.read_u16::<LittleEndian>().unwrap();
                let index = rdr.read_u16::<LittleEndian>().unwrap();
                let data = &input_buffer[begin+6..];
                ServerClientMsg::DataNotify {
                    room,
                    component_type,
                    creator_id,
                    index,
                    data,
                }
            }
            5 => {
                let room = rdr.read_u8().unwrap();
                let creator_id = rdr.read_u16::<LittleEndian>().unwrap();
                let index = rdr.read_u16::<LittleEndian>().unwrap();
                let owner_id = rdr.read_u16::<LittleEndian>().unwrap();
                ServerClientMsg::DataOwner {
                    room,
                    creator_id,
                    index,
                    owner_id,
                }
            }
            type_index => {
                bail!("unsupported msg type: {type_index}");
            }
        };

        Ok(msg)
    }

    pub fn pack(&self, wtr: &mut impl Write) {
        match self {
            ServerClientMsg::Hello { session_id, model } => {
                let mut facts_len = 0;
                for (_, fact) in &model.facts {
                    facts_len += 10;
                    facts_len += fact.len();
                }
                let model_len = 4 + facts_len;
                let len = 6 + model_len;
                wtr.write_u32::<LittleEndian>(len as u32).unwrap();
                wtr.write_u32::<LittleEndian>(0).unwrap();
                wtr.write_u16::<LittleEndian>(*session_id).unwrap();
                wtr.write_u32::<LittleEndian>(model.facts.len() as u32).unwrap();
                for ((room, component_type, creator_id, index), fact) in &model.facts {
                    wtr.write_u8(*room).unwrap();
                    wtr.write_u8(*component_type).unwrap();
                    wtr.write_u16::<LittleEndian>(*creator_id).unwrap();
                    wtr.write_u16::<LittleEndian>(*index).unwrap();
                    let len = fact.len();
                    wtr.write_u32::<LittleEndian>(len as u32).unwrap();
                    wtr.write_all(&fact).unwrap();
                }
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
            ServerClientMsg::InterClient (sender, bytes) => {
                wtr.write_u32::<LittleEndian>(8 + bytes.len() as u32).unwrap();
                wtr.write_u32::<LittleEndian>(3).unwrap();
                wtr.write_u32::<LittleEndian>(*sender as u32).unwrap();
                wtr.write_all(bytes).unwrap();
            }
            ServerClientMsg::DataNotify { room, component_type, creator_id, index, data } => {
                wtr.write_u32::<LittleEndian>(10 + data.len() as u32).unwrap();
                wtr.write_u32::<LittleEndian>(4).unwrap();
                wtr.write_u8(*room).unwrap();
                wtr.write_u8(*component_type).unwrap();
                wtr.write_u16::<LittleEndian>(*creator_id).unwrap();
                wtr.write_u16::<LittleEndian>(*index).unwrap();
                wtr.write_all(data).unwrap();
            }
            ServerClientMsg::DataOwner { room, creator_id, index, owner_id } => {
                wtr.write_u32::<LittleEndian>(11).unwrap();
                wtr.write_u32::<LittleEndian>(5).unwrap();
                wtr.write_u8(*room).unwrap();
                wtr.write_u16::<LittleEndian>(*creator_id).unwrap();
                wtr.write_u16::<LittleEndian>(*index).unwrap();
                wtr.write_u16::<LittleEndian>(*owner_id).unwrap();
            }
        }
    }
}
