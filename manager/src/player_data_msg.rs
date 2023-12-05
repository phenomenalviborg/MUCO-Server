use std::io::{Cursor, Write};

use anyhow::bail;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::player_data::PlayerAttribute;

#[derive(Debug)]
pub enum PlayerDataMsg {
    Notify (PlayerAttribute),
    Set (PlayerAttribute),
    _Request,
}

impl PlayerDataMsg {
    pub fn decode(input_buffer: &[u8], sender: u32) -> anyhow::Result<PlayerDataMsg> {
        let mut rdr = Cursor::new(&input_buffer);
        let msg_type_index = rdr.read_u32::<LittleEndian>().unwrap();

        let begin = 4;

        let msg = match msg_type_index {
            0 => {
                let data = PlayerAttribute::decode(&input_buffer[begin..], sender)?;
                PlayerDataMsg::Notify(data)
            }
            type_index => {
                bail!("unsupported player data msg type: {type_index}");
            }
        };

        Ok(msg)
    }

    pub fn pack(&self, wtr: &mut impl Write) {
        match self {
            PlayerDataMsg::Notify(_) => todo!(),
            PlayerDataMsg::Set(attribute) => {
                wtr.write_u32::<LittleEndian>(1).unwrap();
                attribute.pack(wtr);
            }
            PlayerDataMsg::_Request => todo!(),
        }
    }
}
