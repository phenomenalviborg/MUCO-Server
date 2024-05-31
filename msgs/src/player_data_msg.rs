use std::io::Write;

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
    pub fn decode(rdr: &mut &[u8]) -> anyhow::Result<PlayerDataMsg> {
        let msg_type_index = rdr.read_u32::<LittleEndian>().unwrap();

        let msg = match msg_type_index {
            0 => {
                let data = PlayerAttribute::decode(rdr)?;
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
                tag.pack(wtr);
            }
        }
    }
}
