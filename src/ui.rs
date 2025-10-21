use std::sync::Arc;

use bitcoin::{blocks::BlockChain, client::Client, network::Request, transaction::Transaction};
use tokio::sync::Mutex;

pub struct Ui {
    blockchain: Arc<Mutex<BlockChain>>,
    me: Client,
}

impl Ui {
    pub fn new(blockchain: Arc<Mutex<BlockChain>>, me: Client) -> Self {
        Self {
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
                self.blockchain
                    .lock()
                    .await
                    .new_block(trans),
            ));
        }

        if cmd[0] == "bc" {
            println!(
                "{:?}",
                self.blockchain
                    .lock()
                    .await
            );
        }
        None
    }
}
