use std::sync::{Arc, Weak};

use bitcoin::{
    blocks::{Block, BlockChain},
    client::Client,
    network::{Network, NetworkConfig, Request},
    transaction::Transaction,
};
use tokio::sync::Mutex;

pub struct Ui<'a> {
    network_config: &'a mut NetworkConfig,
    blockchain: Arc<Mutex<BlockChain>>,
    me: &'a Client,
}

impl<'a> Ui<'a> {
    pub fn new(network_config: &'a mut NetworkConfig, blockchain: Arc<Mutex<BlockChain>>, me: &'a Client) -> Self {
        Self {
            network_config,
            blockchain,
            me,
        }
    }

    // TODO: typed command and result propagation
    pub async fn cmd(&mut self, cmd: &str) -> Option<Request> {
        let cmd: Vec<&str> = cmd
            .split_whitespace()
            .collect();
        if cmd[0] == "trans" {
            let trans = Transaction::new(
                *self.me,
                Client::new(
                    cmd[1]
                        .parse()
                        .unwrap(),
                ),
                cmd[2]
                    .parse()
                    .unwrap(),
            );
            return Some(Request::Block(
                self.blockchain
                    .lock()
                    .await
                    .new_block(trans),
            ));
        }
        None
    }
}
