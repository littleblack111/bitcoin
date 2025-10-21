use bincode::Encode;
use serde::{Deserialize, Serialize};

use crate::client::Client;

#[derive(Hash, Deserialize, Serialize, Clone, Copy, PartialEq, Encode)]
pub struct Transaction {
    from: Client,
    to: Client,
    amount: u32,
}

impl Transaction {
    pub fn new(from: Client, to: Client, amount: u32) -> Self {
        Self {
            from,
            to,
            amount,
        }
    }
}
