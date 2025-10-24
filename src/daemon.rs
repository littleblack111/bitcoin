use std::sync::Arc;

use bitcoin::{
    blocks::BlockChain,
    network::{Network, Request},
};
use tokio::sync::Mutex;

pub async fn init() -> Arc<Mutex<Network>> {
    let bc = Arc::new(Mutex::new(BlockChain::default()));
    let network = Network::new(bc).await;

    Network::start(network.clone());

    Network::broadcast(network.clone(), Request::Ibd(None)).await;

    network
}
