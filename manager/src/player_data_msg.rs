use std::io::{Cursor, Write};

use anyhow::bail;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::player_data::{PlayerAttribute, PlayerAttributeTag};

#[derive(Debug)]
pub enum PlayerDataMsg {
    Notify (PlayerAttribute),
    Set (PlayerAttribute),
    Request (PlayerAttributeTag),
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
            PlayerDataMsg::Notify(attribute) => {
                wtr.write_u32::<LittleEndian>(0).unwrap();
                attribute.pack(wtr);
            }
            PlayerDataMsg::Set(attribute) => {
                wtr.write_u32::<LittleEndian>(1).unwrap();
                attribute.pack(wtr);
            }
            PlayerDataMsg::Request (tag) => {
                wtr.write_u32::<LittleEndian>(2).unwrap();
                let tag_index = match tag {
                    PlayerAttributeTag::DeviceId => 0,
                    PlayerAttributeTag::_Color => 1,
                    PlayerAttributeTag::_Trans => 2,
                    PlayerAttributeTag::_Level => 3,
                    PlayerAttributeTag::_Hands => 4,
                    PlayerAttributeTag::_Language => 5,
                    PlayerAttributeTag::_EnvironmentCode => 6,
                    PlayerAttributeTag::_DevMode => 7,
                    PlayerAttributeTag::_IsVisible => 8,
                };
                wtr.write_u32::<LittleEndian>(tag_index).unwrap();
            }
        }
    }
}
