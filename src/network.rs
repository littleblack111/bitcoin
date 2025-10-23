use std::fmt::{Debug, Display};
use std::net::SocketAddr;
use std::sync::{Arc, Weak};

use serde::{Deserialize, Serialize};

use futures::stream::SplitStream;
use futures::{SinkExt, StreamExt};
use tokio::net::ToSocketAddrs;
use tokio::{
    net::{TcpListener, TcpStream},
    spawn,
    sync::{
        Mutex,
        mpsc::{UnboundedSender, unbounded_channel},
    },
};
use tokio_serde::{Framed, formats::Json};
use tokio_util::codec::Framed as TokioFramed;
use tokio_util::codec::LengthDelimitedCodec;

use crate::{
    blocks::{Block, BlockChain},
    client::Client,
    transaction::Transaction,
};

#[derive(Deserialize, Serialize, Clone)]
// Make possible to take borrowed so we dont need to clone everything
pub enum Request {
    Block(Arc<Block>),
    Ibd(Option<Arc<BlockChain>>),
}

#[derive(Default)]
pub struct NetworkConfig {
    auto_mine: bool,
}

pub struct Network {
    me: Client,
    listener: Arc<TcpListener>,
    pub peers: Vec<Peer>,
    blockchain: Arc<Mutex<BlockChain>>,
    config: NetworkConfig,
}

impl Network {
    pub fn get_config(&mut self) -> &mut NetworkConfig {
        &mut self.config
    }
    pub fn get_me(&self) -> &Client {
        &self.me
    }
    pub fn get_blockchain(&self) -> &Arc<Mutex<BlockChain>> {
        &self.blockchain
    }

    pub async fn new(blockchain: Arc<Mutex<BlockChain>>) -> Arc<Mutex<Self>> {
        let listener = TcpListener::bind("0.0.0.0:6767")
            .await
            .unwrap();
        Arc::new(Mutex::new(Self {
            me: Client::default(),
            listener: Arc::new(listener),
            peers: Vec::default(),
            blockchain,
            config: NetworkConfig::default(),
        }))
    }

    pub async fn try_peer(this: Arc<Mutex<Self>>, ip: impl ToSocketAddrs) {
        let stream = TcpStream::connect(ip)
            .await
            .unwrap();
        let weak = Arc::downgrade(&this);
        let peer = Peer::new(weak, stream);
        peer.start();
        {
            this.lock()
                .await
                .peers
                .push(peer);
        }
    }

    pub fn start(this: Arc<Mutex<Self>>) {
        let this_accept = this.clone();
        spawn(async move {
            let listener = {
                this_accept
                    .lock()
                    .await
                    .listener
                    .clone()
            };
            loop {
                let (stream, _addr) = listener
                    .accept()
                    .await
                    .unwrap();
                let peer = Peer::new(Arc::downgrade(&this_accept), stream);
                peer.start();
                {
                    let mut parent = this_accept
                        .lock()
                        .await;
                    parent
                        .peers
                        .push(peer);
                }
            }
        });
        let this_connect = this.clone();
        spawn(async move {
            Network::try_peer(this_connect, "192.168.1.11:6767").await;
        });
        Network::get_idb(this);
    }

    pub fn get_idb(this: Arc<Mutex<Self>>) {
        spawn(async move {
            Network::broadcast(this, Request::Ibd(None)).await;
        });
    }

    pub async fn new_block(this: Arc<Mutex<Self>>, trans: Transaction) -> Block {
        let bc = {
            this.lock()
                .await
                .blockchain
                .clone()
        };
        bc.lock()
            .await
            .new_block(trans)
    }

    pub async fn broadcast(this: Arc<Mutex<Self>>, data: Request) {
        let peers: Vec<UnboundedSender<Request>> = {
            this.lock()
                .await
                .peers
                .iter()
                .map(|p| {
                    p.tx.clone()
                })
                .collect()
        };
        for tx in peers {
            let _ = tx.send(data.clone());
        }
    }
}

pub struct Peer {
    tx: UnboundedSender<Request>,
    addr: SocketAddr,
}

impl Display for Peer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Peer Ip: {}",
            self.addr
                .ip()
        )
    }
}

impl Debug for Peer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self, f)
    }
}

impl Peer {
    fn new(parent: Weak<Mutex<Network>>, stream: TcpStream) -> Self {
        let addr = stream
            .peer_addr()
            .unwrap();
        let framed = Framed::new(TokioFramed::new(stream, LengthDelimitedCodec::new()), Json::default());
        let (sink, stream) = framed.split();
        let (tx, mut rx) = unbounded_channel();
        spawn(async move {
            let mut sink = sink;
            while let Some(msg) = rx
                .recv()
                .await
            {
                let _ = sink
                    .send(msg)
                    .await;
            }
        });
        spawn(async move {
            Peer::read_loop(parent, stream).await;
        });
        Self {
            tx,
            addr,
        }
    }

    fn start(&self) {
        let _ = self
            .tx
            .send(Request::Ibd(None));
    }

    async fn read_loop(parent: Weak<Mutex<Network>>, mut stream: SplitStream<Framed<TokioFramed<TcpStream, LengthDelimitedCodec>, Request, Request, Json<Request, Request>>>) {
        loop {
            while let Some(req) = stream
                .next()
                .await
            {
                if let Ok(msg) = req {
                    if let Some(p) = parent.upgrade() {
                        Peer::handle(p, msg).await;
                    }
                }
            }
        }
    }

    async fn handle(parent: Arc<Mutex<Network>>, req: Request) {
        match req {
            Request::Block(block) => {
                if block
                    .pow
                    .is_some()
                {
                    if !block.verify_pow() {
                        eprintln!("Rejecting remote block, POW verification failed");
                        return;
                    }
                    println!("Accepting and storing remote block: {:#?}", block);
                    let bc = {
                        parent
                            .lock()
                            .await
                            .blockchain
                            .clone()
                    };
                    bc.lock()
                        .await
                        .store((*block).clone())
                } else {
                    println!("Mining new block for transaction: {:#?}", block.trans);
                    let mut mine = (*block).clone();
                    mine.calc_set_pow()
                        .await;
                    {
                        let self_bc = {
                            parent
                                .lock()
                                .await
                                .blockchain
                                .clone()
                        };
                        self_bc
                            .lock()
                            .await
                            .store(mine.clone());
                    }
                    Network::broadcast(parent, Request::Block(Arc::new(mine))).await;
                }
            }
            Request::Ibd(bc) => match bc {
                Some(bc) => {
                    let self_bc = {
                        parent
                            .lock()
                            .await
                            .blockchain
                            .clone()
                    };
                    let mut self_bc = self_bc
                        .lock()
                        .await;
                    if self_bc.is_empty()
                        || (self_bc
                            .blocks
                            .iter()
                            // TODO: Optimize via try_fold
                            .fold(true, |i, b| {
                                if i {
                                    b.pow
                                        .is_none()
                                } else {
                                    false
                                }
                            }))
                    {
                        println!("Setting IBD to {:#?} from remote", bc);
                        *self_bc = (*bc).clone();
                    } else if *self_bc != *bc {
                        eprintln!("Remote IBD broadcast did not match ours, {:#?} vs. {:#?}", bc, self_bc);
                    }
                }
                None => {
                    let bc = {
                        let bc = {
                            parent
                                .lock()
                                .await
                                .blockchain
                                .clone()
                        };
                        let guard = bc
                            .lock()
                            .await;
                        guard.clone()
                    };
                    if bc.is_empty() {
                        return;
                    }
                    println!("Broadcasting as requested"); // TODO: log requester
                    Network::broadcast(parent, Request::Ibd(Some(Arc::new(bc)))).await;
                }
            },
        }
    }
}
