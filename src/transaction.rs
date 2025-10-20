use serde::{Deserialize, Serialize};

use crate::client::Client;

#[derive(Hash, Deserialize, Serialize, Clone, Copy)]
pub struct Transaction {
    from: Client,
    to: Client,
    amount: u32,
}
