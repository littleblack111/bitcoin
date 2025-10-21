use std::{io, sync::Arc};

use bitcoin::{
    blocks::BlockChain,
    network::{Network, Request},
};
use tokio::sync::Mutex;

use crate::ui::Ui;

pub mod ui;

#[tokio::main]
async fn main() {
    let bc = Arc::new(Mutex::new(BlockChain::default()));
    let network = Network::new(bc).await;

    {
        let net = network
            .lock()
            .await;
        net.start();
    }

    Network::broadcast(network.clone(), Request::Ibd(None)).await;

    let (blockchain, me) = {
        let net = network
            .lock()
            .await;
        (
            net.get_blockchain()
                .clone(),
            *net.get_me(),
        )
    };

    let mut ui = Ui::new(blockchain, me);

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
