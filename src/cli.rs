use crate::daemon::init;
use std::{
    io,
    sync::{Arc, Weak},
};

use bitcoin::{
    client::Client,
    network::{Network, Request},
    transaction::Transaction,
};
use tokio::sync::Mutex;

pub mod daemon;

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
            let net = self
                .network
                .upgrade()
                .unwrap();
            let block = Network::new_block(net, trans).await;
            return Some(Request::Block(Arc::new(block)));
        }

        if cmd[0] == "bc" {
            let net = self
                .network
                .upgrade()
                .unwrap();
            let bc = {
                net.lock()
                    .await
                    .get_blockchain()
                    .clone()
            };
            println!(
                "{:#?}",
                bc.lock()
                    .await
            );
        }

        if cmd[0] == "peer" {
            if cmd[1] == "add" {
                let net = self
                    .network
                    .upgrade()
                    .unwrap();
                Network::try_peer(net, cmd[2]).await;
            }

            if cmd[1] == "list" {
                let net = self
                    .network
                    .upgrade()
                    .unwrap();
                println!(
                    "{:#?}",
                    net.lock()
                        .await
                        .peers
                );
            }
        }

        if cmd[0] == "fetch" {
            let net = self
                .network
                .upgrade()
                .unwrap();
            Network::get_idb(net);
        }

        None
    }
}

#[tokio::main]
async fn main() {
    let network = init().await;

    let mut ui = Ui::new(
        Arc::downgrade(&network),
        *network
            .lock()
            .await
            .get_me(),
    );

    loop {
        let mut cmd = String::new();
        io::stdin()
            .read_line(&mut cmd)
            .unwrap();

        if let Some(req) = ui
            .cmd(&cmd)
            .await
        {
            Network::broadcast(network.clone(), req).await;
        }
    }
}
