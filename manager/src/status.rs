use std::collections::HashMap;

use crate::headset_data::{HeadsetData, PersistentHeadsetData, TempHeadsetData, DEFAULT_ENVIRONMENT_CODE, DEFAULT_ENVIRONMENT_NAME};

pub type EnvCodeName = Box<str>;
pub type DeviceId = u32;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Status {
    pub headsets: HashMap<DeviceId, HeadsetData>,
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
        let json = serde_json::to_string_pretty(&persistent_data)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load(path: &str) -> anyhow::Result<Status> {
        let json = std::fs::read_to_string(path)?;
        let persistent_data = serde_json::from_str::<Vec<PersistentHeadsetData>>(&json)?;
        let mut status = Status::new();
        for persistent in persistent_data {
            let k = persistent.unique_device_id;
            let temp = TempHeadsetData::new();
            let v = HeadsetData { persistent, temp };
            status.headsets.insert(k, v);
        }
        Ok(status)
    }
}
