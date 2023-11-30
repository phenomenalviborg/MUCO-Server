use std::io::{Cursor, Write};

use byteorder::{WriteBytesExt, LittleEndian, ReadBytesExt};

#[derive(Debug, Clone)]
pub enum IntercomMsg {
    NewClient (usize),
    ClientServerMsg (usize, ClientServerMsg)
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
    Unsupported (u32),
}

impl ClientServerMsg {
    pub fn dequeue(input_buffer: &mut Vec<u8>) -> Option<ClientServerMsg> {
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

        let msg_type_index = rdr.read_u32::<LittleEndian>().unwrap();

        let begin = 8;

        let msg = Some(match msg_type_index {
            0 => {
                ClientServerMsg::Disconnect
            }
            2 => {
                let bs = input_buffer[begin..end].to_vec();
                ClientServerMsg::BroadcastBytesAll (bs)
            }
            3 => {
                let bs = input_buffer[begin..end].to_vec();
                ClientServerMsg::BroadcastBytesOther (bs)
            }
            6 => {
                let address = rdr.read_u32::<LittleEndian>().unwrap() as usize;
                let bs = input_buffer[begin+4..end].to_vec();
                ClientServerMsg::BinaryMessageTo (address, bs)
            }
            _ => {
                ClientServerMsg::Unsupported (msg_type_index)
            }
        });

        input_buffer.drain(..end);

        msg
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
