use std::io::Write;

use anyhow::bail;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::player_data_msg::PlayerDataMsg;

#[derive(Debug)]
pub enum InterClientMsg {
    _Interaction,
    PlayerData (PlayerDataMsg),
    _Ping,
    AllPlayerData (Vec<u8>),
    Diff (Vec<u8>),
}

impl InterClientMsg {
    pub fn decode(rdr: &mut &[u8]) -> anyhow::Result<InterClientMsg> {
        let msg_type_index = rdr.read_u32::<LittleEndian>().unwrap();

        let msg = match msg_type_index {
            0 => InterClientMsg::_Interaction,
            1 => {
                let player_data_msg = PlayerDataMsg::decode(rdr)?;
                InterClientMsg::PlayerData(player_data_msg)
            }
            2 => InterClientMsg::_Ping,
            3 => InterClientMsg::AllPlayerData (rdr.to_owned()),
            4 => InterClientMsg::Diff (rdr.to_owned()),
            type_index => {
                bail!("unsupported inter client msg type: {type_index}");
            }
        };

        Ok(msg)
    }

    pub fn pack(&self, wtr: &mut impl Write) {
        match self {
            InterClientMsg::_Interaction => {
                todo!()
            }
            InterClientMsg::PlayerData(player_data_msg) => {
                wtr.write_u32::<LittleEndian>(1).unwrap();
                player_data_msg.pack(wtr);
            }
            InterClientMsg::_Ping => {
                todo!()
            }
            InterClientMsg::AllPlayerData (_data) => {
                todo!();
            }
            InterClientMsg::Diff (_diff) => {
                todo!();
            }
        }
    }
}
