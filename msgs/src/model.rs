use std::collections::HashMap;

pub struct SharedData {
    pub model: Model,
    pub data_owners: HashMap<(u8, u16, u16), u16>,
}

impl SharedData {
    pub fn new() -> SharedData {
        SharedData {
            model: Model::new(),
            data_owners: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Model {
    pub facts: HashMap<(u8, u16, u16), Box<[u8]>>,
}

impl Model {
    pub fn new() -> Model {
        Model {
            facts: HashMap::new(),
        }
    }
}
