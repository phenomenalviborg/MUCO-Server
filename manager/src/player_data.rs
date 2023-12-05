use std::io::{Cursor, Write};

use anyhow::bail;
use byteorder::{LittleEndian, ReadBytesExt};

#[derive(Debug)]
pub enum PlayerData {
    Trans,
    Hands,
    Color,
}

impl PlayerData {
    pub fn decode(input_buffer: &[u8], sender: u32) -> anyhow::Result<PlayerData> {
        let mut rdr = Cursor::new(&input_buffer);
        let msg_type_index = rdr.read_u32::<LittleEndian>().unwrap();

        let begin = 4;

        let msg = match msg_type_index {
            0 => PlayerData::Trans,
            1 => PlayerData::Hands,
            2 => PlayerData::Color,
            type_index => {
                bail!("unsupported player data type: {type_index}");
            }
        };

        Ok(msg)
    }

    pub fn pack(&self, wtr: &mut impl Write) {
        match self {
            PlayerData::Trans => todo!(),
            PlayerData::Hands => todo!(),
            PlayerData::Color => todo!(),
        }
    }
}
