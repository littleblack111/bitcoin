use std::{sync::Arc, thread::sleep, time::Duration};

use bitcoin::{blocks::BlockChain, network::Network};
use tokio::sync::Mutex;

#[tokio::main]
async fn main() {
    let bc = Arc::new(Mutex::new(BlockChain::default()));
    let network = Network::new(bc).await;
    network
        .lock()
        .await
        .broadcast(bitcoin::network::Request::Ibd(None))
        .await;
    sleep(Duration::from_secs(100));
}
