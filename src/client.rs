use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Hash, Deserialize, Serialize, Clone, Copy)]
pub struct Client {
    id: u32,
}

impl Default for Client {
    fn default() -> Self {
        Self {
            id: rand::rng().random(),
        }
    }
}
