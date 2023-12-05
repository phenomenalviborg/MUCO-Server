use std::io::{Cursor, Write};

use anyhow::bail;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::player_data_msg::PlayerDataMsg;

#[derive(Debug)]
pub enum InterClientMsg {
    Interaction,
    PlayerData (PlayerDataMsg),
    Ping,
}

impl InterClientMsg {
    pub fn decode(input_buffer: &[u8], sender: u32) -> anyhow::Result<InterClientMsg> {
        let mut rdr = Cursor::new(&input_buffer);
        let msg_type_index = rdr.read_u32::<LittleEndian>().unwrap();

        let begin = 4;

        let msg = match msg_type_index {
            1 => {
                let player_data_msg = PlayerDataMsg::decode(&input_buffer[begin..], sender)?;
                InterClientMsg::PlayerData(player_data_msg)
            }
            type_index => {
                bail!("unsupported inter client msg type: {type_index}");
            }
        };

        Ok(msg)
    }

    pub fn pack(&self, wtr: &mut impl Write) {
        match self {
            InterClientMsg::Interaction => {
                todo!()
            }
            InterClientMsg::PlayerData(player_data_msg) => {
                wtr.write_u32::<LittleEndian>(1).unwrap();
                player_data_msg.pack(wtr);
            }
            InterClientMsg::Ping => {
                todo!()
            }
        }
    }
}
