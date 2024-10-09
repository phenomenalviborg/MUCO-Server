use std::collections::HashMap;

use crate::headset_data::{HeadsetData, PersistentHeadsetData, TempHeadsetData, DEFAULT_ENVIRONMENT_CODE, DEFAULT_ENVIRONMENT_NAME};

pub type EnvCodeName = Box<str>;
pub type DeviceId = u32;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Status {
    pub headsets: HashMap<DeviceId, HeadsetData>,
    pub environment_codes: HashMap<EnvCodeName, Box<str>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SaveData {
    pub headsets: Vec<PersistentHeadsetData>,
    pub environment_codes: HashMap<EnvCodeName, Box<str>>,
}

impl Status {
    pub fn new() -> Status {
        let mut environment_codes = HashMap::new();
        environment_codes.insert(DEFAULT_ENVIRONMENT_NAME.into(), DEFAULT_ENVIRONMENT_CODE.into());
        Status {
            headsets: HashMap::new(),
            environment_codes,
        }
    }

    pub fn save(&self, path: &str) -> anyhow::Result<()>{
        let persistent_data = self.headsets.iter().map(|(_, headset_data)| headset_data.persistent.clone()).collect::<Vec<_>>();
        let save_data = SaveData {
            headsets: persistent_data,
            environment_codes: self.environment_codes.clone(),
        };
        let json = serde_json::to_string_pretty(&save_data)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load(path: &str) -> anyhow::Result<Status> {
        let json = std::fs::read_to_string(path)?;
        let save_data = serde_json::from_str::<SaveData>(&json)?;
        let mut status = Status::new();
        for persistent in save_data.headsets {
            let k = persistent.unique_device_id;
            let temp = TempHeadsetData::new();
            let v = HeadsetData { persistent, temp };
            status.headsets.insert(k, v);
        }
        status.environment_codes = save_data.environment_codes;
        Ok(status)
    }
}
