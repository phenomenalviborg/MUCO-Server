use std::io::{Cursor, Read, Write};

use anyhow::bail;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::{dequeue::dequeue_msg, model::Model};

#[derive(Debug, Clone)]
pub enum ServerClientMsg<'a> {
    Hello {
        session_id: u32,
        model: Model,
    },
    ClientConnected (u32),
    ClientDisconnected (u32),
    InterClient (u32, &'a[u8]),
    DataNotify (u64, &'a[u8]),
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
                let session_id = rdr.read_u32::<LittleEndian>().unwrap();
                let mut model = Model::new();
                let fact_count = rdr.read_u32::<LittleEndian>().unwrap();
                for _ in 0..fact_count {
                    let key = rdr.read_u64::<LittleEndian>().unwrap();
                    let len = rdr.read_u32::<LittleEndian>().unwrap();
                    let mut buffer = vec![0u8; len as usize].into_boxed_slice();
                    rdr.read_exact(&mut buffer).unwrap();
                    model.facts.insert(key, buffer);
                }
                ServerClientMsg::Hello {
                    session_id,
                    model,
                }
            }
            1 => {
                let session_id = rdr.read_u32::<LittleEndian>().unwrap();
                ServerClientMsg::ClientConnected (session_id)
            }
            2 => {
                let session_id = rdr.read_u32::<LittleEndian>().unwrap();
                ServerClientMsg::ClientDisconnected (session_id)
            }
            3 => {
                let sender = rdr.read_u32::<LittleEndian>().unwrap();
                let bs = &input_buffer[begin+4..];
                ServerClientMsg::InterClient (sender, bs)
            }
            4 => {
                let key = rdr.read_u64::<LittleEndian>().unwrap();
                let bs = &input_buffer[begin+8..];
                ServerClientMsg::DataNotify (key, bs)
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
                    facts_len += 12;
                    facts_len += fact.len();
                }
                let model_len = 4 + facts_len;
                let len = 8 + model_len;
                wtr.write_u32::<LittleEndian>(len as u32).unwrap();
                wtr.write_u32::<LittleEndian>(0).unwrap();
                wtr.write_u32::<LittleEndian>(*session_id).unwrap();
                wtr.write_u32::<LittleEndian>(model.facts.len() as u32).unwrap();
                for (key, fact) in &model.facts {
                    wtr.write_u64::<LittleEndian>(*key).unwrap();
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
            ServerClientMsg::DataNotify (key, data) => {
                wtr.write_u32::<LittleEndian>(12 + data.len() as u32).unwrap();
                wtr.write_u32::<LittleEndian>(4).unwrap();
                wtr.write_u64::<LittleEndian>(*key).unwrap();
                wtr.write_all(data).unwrap();
            }
        }
    }
}
