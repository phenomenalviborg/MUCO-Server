use std::collections::HashMap;

use crate::headset_data::HeadsetData;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Status {
    pub headsets: HashMap<String, HeadsetData>,
}

impl Status {
    pub fn new() -> Status {
        Status {
            headsets: HashMap::new(),
        }
    }

    pub fn save(&self, path: &str) -> anyhow::Result<()>{
        let json = serde_json::to_string(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load(path: &str) -> anyhow::Result<Status> {
        let json = std::fs::read_to_string(path)?;
        let status = serde_json::from_str::<Status>(&json)?;
        Ok(status)
    }
}

