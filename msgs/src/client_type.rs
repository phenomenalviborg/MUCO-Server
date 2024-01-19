
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClientType {
    Player,
    Manager,
}

impl ClientType {
    pub fn from_u32(index: u32) -> Option<ClientType> {
        match index {
            0 => Some(ClientType::Player),
            1 => Some(ClientType::Manager),
            _ => None,
        }
    }

    pub fn as_u32(&self) -> u32 {
        match self {
            ClientType::Player => 0,
            ClientType::Manager => 1,
        }
    }
}
