use std::collections::HashMap;

use crate::headset_data::{HeadsetData, PersistentHeadsetData, TempHeadsetData};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Status {
    pub headsets: HashMap<String, HeadsetData>,
    pub environment_codes: HashMap<String, String>,
}

impl Status {
    pub fn new() -> Status {
        Status {
            headsets: HashMap::new(),
            environment_codes: HashMap::new(),
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
            let k = persistent.unique_device_id.to_string();
            let temp = TempHeadsetData::new();
            let v = HeadsetData { persistent, temp };
            status.headsets.insert(k, v);
        }
        Ok(status)
    }
}
