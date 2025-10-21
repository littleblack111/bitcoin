use std::{io, sync::Arc};

use bitcoin::{blocks::BlockChain, network::Network};
use tokio::sync::Mutex;

use crate::ui::Ui;

pub mod ui;

#[tokio::main]
async fn main() {
    let bc = Arc::new(Mutex::new(BlockChain::default()));
    let network = Network::new(bc).await;
    network
        .lock()
        .await
        .start();
    network
        .lock()
        .await
        .broadcast(bitcoin::network::Request::Ibd(None))
        .await;
    let n = network
        .lock()
        .await;
    let mut binding = network
        .lock()
        .await;
    let mut ui = Ui::new(
        binding.get_config(),
        n.get_blockchain()
            .clone(),
        n.get_me(),
    );
    loop {
        let mut cmd = String::new();
        io::stdin()
            .read_line(&mut cmd)
            .unwrap();

        ui.cmd(&cmd)
            .await;
    }
}
