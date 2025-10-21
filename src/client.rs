use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Hash, Deserialize, Serialize, Clone, Copy, PartialEq)]
pub struct Client {
    id: u32,
}

impl Client {
    pub fn new(id: u32) -> Self {
        Self {
            id,
        }
    }
}

impl Default for Client {
    fn default() -> Self {
        Self {
            id: rand::rng().random(),
        }
    }
}
