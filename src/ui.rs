use std::sync::{Arc, Weak};

use bitcoin::{
    blocks::BlockChain,
    client::Client,
    network::{Network, Request},
    transaction::Transaction,
};
use tokio::sync::Mutex;

pub struct Ui {
    network: Weak<Mutex<Network>>,
    me: Client,
}

impl Ui {
    pub fn new(network: Weak<Mutex<Network>>, me: Client) -> Self {
        Self {
            network,
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
                self.me,
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
                self.network
                    .upgrade()
                    .unwrap()
                    .lock()
                    .await
                    .get_blockchain()
                    .lock()
                    .await
                    .new_block(trans),
            ));
        }

        if cmd[0] == "bc" {
            println!(
                "{:#?}",
                self.network
                    .upgrade()
                    .unwrap()
                    .lock()
                    .await
                    .get_blockchain()
                    .lock()
                    .await
            );
        }

        if cmd[0] == "peer" {
            if cmd[1] == "add" {
                Network::try_peer(
                    self.network
                        .clone(),
                    cmd[2],
                )
                .await;
            }
        }

        None
    }
}
