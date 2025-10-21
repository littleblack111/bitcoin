use serde::{Deserialize, Serialize};

use crate::client::Client;

#[derive(Hash, Deserialize, Serialize, Clone, Copy, PartialEq)]
pub struct Transaction {
    from: Client,
    to: Client,
    amount: u32,
}
