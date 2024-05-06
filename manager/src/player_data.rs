use std::{io::{Cursor, Read, Write}, vec};

use anyhow::bail;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::color::Color;

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum Language {
    EnGB,
    DaDK,
    DeDE,
}

#[derive(Debug)]
pub enum PlayerAttribute {
    DeviceId (u32),
    Color (Color),
    Trans,
    Level,
    Hands,
    Language (Language),
    EnvironmentCode (String),
    DevMode (bool),
    IsVisible (bool),
}

#[derive(Debug)]
pub enum PlayerAttributeTag {
    DeviceId,
    _Color,
    _Trans,
    _Level,
    _Hands,
    _Language,
    _EnvironmentCode,
    _DevMode,
    _IsVisible,
}

impl PlayerAttribute {
    pub fn decode(input_buffer: &[u8], _sender: u32) -> anyhow::Result<PlayerAttribute> {
        let mut rdr = Cursor::new(&input_buffer);
        let msg_type_index = rdr.read_u32::<LittleEndian>().unwrap();

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
            3 => PlayerAttribute::Level,
            4 => PlayerAttribute::Hands,
            5 => {
                let language_index = rdr.read_u32::<LittleEndian>().unwrap();
                let language = match language_index {
                    0 => Language::EnGB,
                    1 => Language::DaDK,
                    2 => Language::DeDE,
                    _ => bail!("unsupported language index: {language_index}")
                };
                PlayerAttribute::Language(language)
            }
            6 => {
                let len = rdr.read_u32::<LittleEndian>().unwrap();
                let mut buf = vec![0u8; len as usize];
                rdr.read(&mut buf).unwrap();
                let code = String::from_utf8(buf).unwrap();
                PlayerAttribute::EnvironmentCode(code)
            }
            7 => {
                let x = rdr.read_u8().unwrap();
                let in_dev_mode = match x {
                    0 => false,
                    _ => true,
                };
                PlayerAttribute::DevMode(in_dev_mode)
            }
            8 => {
                let x = rdr.read_u8().unwrap();
                let in_dev_mode = match x {
                    0 => false,
                    _ => true,
                };
                PlayerAttribute::DevMode(in_dev_mode)
            }
            9 => {
                let x = rdr.read_u8().unwrap();
                let is_visible = match x {
                    0 => false,
                    _ => true,
                };
                PlayerAttribute::IsVisible(is_visible)
            }
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
            PlayerAttribute::Level => todo!(),
            PlayerAttribute::Hands => todo!(),
            PlayerAttribute::Language (language) => {
                wtr.write_u32::<LittleEndian>(5).unwrap();
                let language_index = match language {
                    Language::EnGB => 0,
                    Language::DaDK => 1,
                    Language::DeDE => 2,
                };
                wtr.write_u32::<LittleEndian>(language_index).unwrap();
            }
            PlayerAttribute::EnvironmentCode(code) => {
                wtr.write_u32::<LittleEndian>(6).unwrap();
                wtr.write_u32::<LittleEndian>(code.len() as u32).unwrap();
                wtr.write_all(code.as_bytes()).unwrap();
            }
            PlayerAttribute::DevMode(is_on) => {
                wtr.write_u32::<LittleEndian>(7).unwrap();
                let buffer = if *is_on { &[1] } else { &[0] };
                wtr.write_all(buffer).unwrap();
            }
            PlayerAttribute::IsVisible(is_visible) => {
                wtr.write_u32::<LittleEndian>(8).unwrap();
                let buffer = if *is_visible { &[1] } else { &[0] };
                wtr.write_all(buffer).unwrap();
            }
        }
    }
}
