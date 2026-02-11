use std::collections::HashMap;

use msgs::player_data::{EnvData, EnvTrans, GuardianConfig};

use crate::headset_data::{HeadsetData, PersistentHeadsetData, TempHeadsetData, DEFAULT_ENVIRONMENT_CODE, DEFAULT_ENVIRONMENT_NAME};

pub type EnvCodeName = Box<str>;
pub type DeviceId = u32;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Status {
    pub headsets: HashMap<DeviceId, HeadsetData>,
    pub environment_data: HashMap<EnvCodeName, EnvData>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SaveData {
    pub headsets: Vec<PersistentHeadsetData>,
    pub environment_data: HashMap<EnvCodeName, EnvData>,
}

impl Status {
    pub fn new() -> Status {
        let mut environment_data = HashMap::new();
        let default_env_data = EnvData {
            code: DEFAULT_ENVIRONMENT_CODE.into(),
            transform: EnvTrans::default(),
            guardian: GuardianConfig::default(),
        };
        environment_data.insert(DEFAULT_ENVIRONMENT_NAME.into(), default_env_data.into());
        Status {
            headsets: HashMap::new(),
            environment_data,
        }
    }

    pub fn save(&self, path: &str) -> anyhow::Result<()>{
        let persistent_data = self.headsets.iter().map(|(_, headset_data)| headset_data.persistent.clone()).collect::<Vec<_>>();
        let save_data = SaveData {
            headsets: persistent_data,
            environment_data: self.environment_data.clone(),
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
        status.environment_data = save_data.environment_data;
        Ok(status)
    }
}
