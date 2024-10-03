use std::{io::{Read, Write}, vec};

use anyhow::bail;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::color::Color;

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum Language {
    EnGB,
    DaDK,
    DeDE,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DeviceStats {
    pub battery_status: BatteryStatus,
    pub battery_level: f32,
    pub fps: f32,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum BatteryStatus {
    Unknown,
    Charging,
    Discharging,
    NotCharging,
    Full,
}

#[derive(Debug)]
pub enum PlayerAttribute {
    DeviceId (u32),
    Color (Color),
    Trans,
    Level (f32),
    Hands,
    Language (Language),
    EnvironmentCode (Box<str>),
    DevMode (bool),
    IsVisible (bool),
    DeviceStats (DeviceStats),
    AudioVolume (f32),
}

#[derive(Debug, Clone, Copy)]
pub enum PlayerAttributeTag {
    DeviceId,
    Color,
    Trans,
    Level,
    Hands,
    Language,
    EnvironmentCode,
    DevMode,
    IsVisible,
    DeviceStats,
    AudioVolume,
}

impl PlayerAttributeTag {
    pub const ALL_TAGS: &'static [PlayerAttributeTag] = &[
        PlayerAttributeTag::DeviceId,
        PlayerAttributeTag::Color,
        PlayerAttributeTag::Trans,
        PlayerAttributeTag::Level,
        PlayerAttributeTag::Hands,
        PlayerAttributeTag::Language,
        PlayerAttributeTag::EnvironmentCode,
        PlayerAttributeTag::DevMode,
        PlayerAttributeTag::IsVisible,
        PlayerAttributeTag::DeviceStats,
        PlayerAttributeTag::AudioVolume,
    ];

    pub fn decode(rdr: &mut &[u8]) -> anyhow::Result<Self> {
        let tag_index = rdr.read_u32::<LittleEndian>()?;
        let tag = match tag_index {
            0 => PlayerAttributeTag::DeviceId,
            1 => PlayerAttributeTag::Color,
            2 => PlayerAttributeTag::Trans,
            3 => PlayerAttributeTag::Level,
            4 => PlayerAttributeTag::Hands,
            5 => PlayerAttributeTag::Language,
            6 => PlayerAttributeTag::EnvironmentCode,
            7 => PlayerAttributeTag::DevMode,
            8 => PlayerAttributeTag::IsVisible,
            9 => PlayerAttributeTag::DeviceStats,
            10 => PlayerAttributeTag::AudioVolume,
            _ => bail!("tag index not supported")
        };
        Ok(tag)
    }
    pub fn pack(&self, wtr: &mut impl Write) {
        let tag_index = match self {
            PlayerAttributeTag::DeviceId => 0,
            PlayerAttributeTag::Color => 1,
            PlayerAttributeTag::Trans => 2,
            PlayerAttributeTag::Level => 3,
            PlayerAttributeTag::Hands => 4,
            PlayerAttributeTag::Language => 5,
            PlayerAttributeTag::EnvironmentCode => 6,
            PlayerAttributeTag::DevMode => 7,
            PlayerAttributeTag::IsVisible => 8,
            PlayerAttributeTag::DeviceStats => 9,
            PlayerAttributeTag::AudioVolume => 10,
        };
        wtr.write_u32::<LittleEndian>(tag_index).unwrap();
    }
}

impl PlayerAttribute {
    pub const TRANS_SIZE: usize = 28;
    pub const LEVEL_SIZE: usize = 4;

    pub fn decode(rdr: &mut &[u8]) -> anyhow::Result<PlayerAttribute> {
        let tag = PlayerAttributeTag::decode(rdr)?;
        Self::decode_(rdr, tag)
    }

    pub fn decode_(rdr: &mut &[u8], tag: PlayerAttributeTag) -> anyhow::Result<PlayerAttribute> {
        let msg = match tag {
            PlayerAttributeTag::DeviceId => {
                let device_id = rdr.read_u32::<LittleEndian>().unwrap();
                PlayerAttribute::DeviceId(device_id)
            }
            PlayerAttributeTag::Color => {
                let r = rdr.read_f32::<LittleEndian>().unwrap();
                let g = rdr.read_f32::<LittleEndian>().unwrap();
                let b = rdr.read_f32::<LittleEndian>().unwrap();
                let a = rdr.read_f32::<LittleEndian>().unwrap();
                let color = Color { r, g, b, a };
                PlayerAttribute::Color(color)
            }
            PlayerAttributeTag::Trans => {
                *rdr = &rdr[Self::TRANS_SIZE..];
                PlayerAttribute::Trans
            }
            PlayerAttributeTag::Level => {
                let level = rdr.read_f32::<LittleEndian>()?;
                PlayerAttribute::Level (level)
            }
            PlayerAttributeTag::Hands => {
                let _hand_type = rdr.read_u8()?;
                let _left_hand_confidence = rdr.read_u8()?;
                let _right_hand_confidence = rdr.read_u8()?;

                *rdr = &rdr[Self::TRANS_SIZE..];
                *rdr = &rdr[Self::TRANS_SIZE..];

                let trans_count = rdr.read_u32::<LittleEndian>()?;
                let len = trans_count as usize * Self::TRANS_SIZE;
                *rdr = &rdr[len..];

                let trans_count = rdr.read_u32::<LittleEndian>()?;
                let len = trans_count as usize * Self::TRANS_SIZE;
                *rdr = &rdr[len..];
                PlayerAttribute::Hands
            }
            PlayerAttributeTag::Language => {
                let language_index = rdr.read_u32::<LittleEndian>().unwrap();
                let language = match language_index {
                    0 => Language::EnGB,
                    1 => Language::DaDK,
                    2 => Language::DeDE,
                    _ => bail!("unsupported language index: {language_index}")
                };
                PlayerAttribute::Language(language)
            }
            PlayerAttributeTag::EnvironmentCode => {
                let len = rdr.read_u32::<LittleEndian>().unwrap();
                let mut buf = vec![0u8; len as usize];
                rdr.read(&mut buf).unwrap();
                let code = String::from_utf8(buf).unwrap().into();
                PlayerAttribute::EnvironmentCode(code)
            }
            PlayerAttributeTag::DevMode => {
                let x = rdr.read_u8().unwrap();
                let in_dev_mode = match x {
                    0 => false,
                    _ => true,
                };
                PlayerAttribute::DevMode(in_dev_mode)
            }
            PlayerAttributeTag::IsVisible => {
                let x = rdr.read_u8().unwrap();
                let is_visible = match x {
                    0 => false,
                    _ => true,
                };
                PlayerAttribute::IsVisible(is_visible)
            }
            PlayerAttributeTag::DeviceStats => {
                let status_byte = rdr.read_u8()?;
                let device_stats = DeviceStats {
                    battery_status: match status_byte {
                        0 => BatteryStatus::Unknown,
                        1 => BatteryStatus::Charging,
                        2 => BatteryStatus::Discharging,
                        3 => BatteryStatus::NotCharging,
                        4 => BatteryStatus::Full,
                        _ => bail!("unknown battery status")
                    },
                    battery_level: rdr.read_f32::<LittleEndian>()?,
                    fps: rdr.read_f32::<LittleEndian>()?,
                };
                PlayerAttribute::DeviceStats(device_stats)
            }
            PlayerAttributeTag::AudioVolume => {
                let audio_volume = rdr.read_f32::<LittleEndian>()?;
                PlayerAttribute::AudioVolume(audio_volume)
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
            PlayerAttribute::Level (level) => {
                wtr.write_u32::<LittleEndian>(3).unwrap();
                wtr.write_f32::<LittleEndian>(*level).unwrap();
            },
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
            PlayerAttribute::DeviceStats(_) => todo!(),
            PlayerAttribute::AudioVolume(audio_volume) => {
                wtr.write_u32::<LittleEndian>(10).unwrap();
                wtr.write_f32::<LittleEndian>(*audio_volume).unwrap();
            }
        }
    }
}
