use std::io::{Cursor, Write};

use anyhow::bail;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::color::Color;

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum Language {
    English,
    Dansk,
    Deutsch,
}

#[derive(Debug)]
pub enum PlayerAttribute {
    DeviceId (u32),
    Color (Color),
    Trans,
    Hands,
    Language (Language),
    EnvironmentCode (String),
}

#[derive(Debug)]
pub enum PlayerAttributeTag {
    DeviceId,
    _Color,
    _Trans,
    _Hands,
    _Language,
    _EnvironmentCode,
}

impl PlayerAttribute {
    pub fn decode(input_buffer: &[u8], _sender: u32) -> anyhow::Result<PlayerAttribute> {
        let mut rdr = Cursor::new(&input_buffer);
        let msg_type_index = rdr.read_u32::<LittleEndian>().unwrap();

        //let begin = 4;

        let msg = match msg_type_index {
            0 => {
                let device_id = rdr.read_u32::<LittleEndian>().unwrap();
                PlayerAttribute::DeviceId(device_id)
            }
            1 => {
                let r = rdr.read_f32::<LittleEndian>().unwrap();
                let g = rdr.read_f32::<LittleEndian>().unwrap();
                let b = rdr.read_f32::<LittleEndian>().unwrap();
                let a = rdr.read_f32::<LittleEndian>().unwrap();
                let color = Color { r, g, b, a };
                PlayerAttribute::Color(color)
            }
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
            PlayerAttribute::Color (color) => {
                wtr.write_u32::<LittleEndian>(1).unwrap();
                wtr.write_f32::<LittleEndian>(color.r).unwrap();
                wtr.write_f32::<LittleEndian>(color.g).unwrap();
                wtr.write_f32::<LittleEndian>(color.b).unwrap();
                wtr.write_f32::<LittleEndian>(color.a).unwrap();
            }
            PlayerAttribute::Trans => todo!(),
            PlayerAttribute::Hands => todo!(),
            PlayerAttribute::Language (language) => {
                wtr.write_u32::<LittleEndian>(4).unwrap();
                let language_index = match language {
                    Language::English => 0,
                    Language::Dansk => 1,
                    Language::Deutsch => 2,
                };
                wtr.write_u32::<LittleEndian>(language_index).unwrap();
            }
            PlayerAttribute::EnvironmentCode(code) => {
                wtr.write_u32::<LittleEndian>(5).unwrap();
                wtr.write_u32::<LittleEndian>(code.len() as u32).unwrap();
                wtr.write(code.as_bytes()).unwrap();
            }
        }
    }
}
