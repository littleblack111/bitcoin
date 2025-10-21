use std::{sync::Arc, time::Duration};

use bitcoin::{blocks::BlockChain, network::Network};
use tokio::sync::Mutex;

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
    tokio::time::sleep(Duration::from_secs(100)).await;
}
