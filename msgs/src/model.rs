use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Model {
    pub facts: HashMap<u64, Box<[u8]>>,
}

impl Model {
    pub fn new() -> Model {
        Model {
            facts: HashMap::new()
        }
    }
}
