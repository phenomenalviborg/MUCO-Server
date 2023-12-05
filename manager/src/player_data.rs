use std::io::{Cursor, Write};

use anyhow::bail;
use byteorder::{LittleEndian, ReadBytesExt};

#[derive(Debug)]
pub enum PlayerAttribute {
    DeviceId (u32),
    Color,
    Trans,
    Hands,
}

impl PlayerAttribute {
    pub fn decode(input_buffer: &[u8], sender: u32) -> anyhow::Result<PlayerAttribute> {
        let mut rdr = Cursor::new(&input_buffer);
        let msg_type_index = rdr.read_u32::<LittleEndian>().unwrap();

        let begin = 4;

        let msg = match msg_type_index {
            0 => {
                let device_id = rdr.read_u32::<LittleEndian>().unwrap();
                PlayerAttribute::DeviceId(device_id)
            }
            1 => PlayerAttribute::Color,
            2 => PlayerAttribute::Trans,
            3 => PlayerAttribute::Hands,
            type_index => {
                bail!("unsupported player data type: {type_index}");
            }
        };

        Ok(msg)
    }

    pub fn pack(&self, wtr: &mut impl Write) {
        match self {
            PlayerAttribute::DeviceId (_) => todo!(),
            PlayerAttribute::Color => todo!(),
            PlayerAttribute::Trans => todo!(),
            PlayerAttribute::Hands => todo!(),
        }
    }
}
