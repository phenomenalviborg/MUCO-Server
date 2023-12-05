use std::io::{Cursor, Write};

use anyhow::bail;
use byteorder::{LittleEndian, ReadBytesExt};

pub enum InterClientMsg {
    Interaction,
    PlayerData (PlayerDataMsg),
    Ping,
}

pub enum PlayerData {
    Trans,
    Hands,
    Color,
}

pub enum PlayerDataMsg {
    Notify (PlayerData),
    Set (PlayerData),
    Request,
}

impl InterClientMsg {
    pub fn decode(input_buffer: &[u8], sender: u32) -> anyhow::Result<InterClientMsg> {
        let mut rdr = Cursor::new(&input_buffer);
        let msg_type_index = rdr.read_u32::<LittleEndian>().unwrap();

        let begin = 4;

        let msg = match msg_type_index {
            type_index => {
                bail!("unsupported msg type: {type_index}");
            }
        };

        Ok(msg)
    }

    pub fn pack(&self, wtr: &mut impl Write) {
        match self {
            InterClientMsg::Interaction => {
                todo!()
            }
            InterClientMsg::PlayerData(_) => {
                todo!()
            }
            InterClientMsg::Ping => {
                todo!()
            }
        }
    }
}
