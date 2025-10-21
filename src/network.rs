use std::sync::{Arc, Weak};

use serde::{Deserialize, Serialize};

use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use tokio::net::ToSocketAddrs;
use tokio::{
    net::{TcpListener, TcpStream},
    spawn,
    sync::Mutex,
};
use tokio_serde::{Framed, formats::Json};
use tokio_util::codec::Framed as TokioFramed;
use tokio_util::codec::LengthDelimitedCodec;

use crate::{
    blocks::{Block, BlockChain},
    client::Client,
};

#[derive(Deserialize, Serialize, Clone)]
// Make possible to take borrowed so we dont need to clone everything
pub enum Request {
    Block(Block),
    Ibd(Option<BlockChain>),
}

#[derive(Default)]
pub struct NetworkConfig {
    auto_mine: bool,
}

pub struct Network {
    this: Weak<Mutex<Self>>,
    me: Client,

    listener: Arc<TcpListener>,
    peers: Vec<Arc<Mutex<Peer>>>,
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
        let this = Arc::new(Mutex::new(Self {
            me: Client::default(),
            this: Weak::new(),
            listener: Arc::new(listener),
            peers: Vec::default(),
            blockchain,
            config: NetworkConfig::default(),
        }));
        this.lock()
            .await
            .this = Arc::downgrade(&this);
        this
    }

    pub async fn try_peer(this: Weak<Mutex<Self>>, ip: impl ToSocketAddrs) {
        let stream = TcpStream::connect(ip)
            .await
            .unwrap();
        let peer = Arc::new(Mutex::new(Peer::new(this.clone(), stream)));
        if let Some(parent) = this.upgrade() {
            parent
                .lock()
                .await
                .peers
                .push(Arc::clone(&peer));
        }
        spawn(async move {
            peer.lock()
                .await
                .start()
                .await;
        });
    }

    pub fn start(&self) {
        let this = self
            .this
            .clone();
        let listener = self
            .listener
            .clone();
        spawn(async move {
            loop {
                let (stream, _addr) = listener
                    .accept()
                    .await
                    .unwrap();

                let peer = Arc::new(Mutex::new(Peer::new(this.clone(), stream)));

                if let Some(parent) = this.upgrade() {
                    let mut parent = parent
                        .lock()
                        .await;
                    parent
                        .peers
                        .push(Arc::clone(&peer));
                }

                spawn({
                    let peer = Arc::clone(&peer);
                    async move {
                        peer.lock()
                            .await
                            .start()
                            .await;
                    }
                });
            }
        });
        let this = self
            .this
            .clone();
        spawn(async move {
            Network::try_peer(this, "192.168.1.16:6767").await;
        });

        self.get_idb()
    }

    pub fn get_idb(&self) {
        let this = self
            .this
            .clone();
        spawn(async move {
            if let Some(parent) = this.upgrade() {
                Network::broadcast(parent, Request::Ibd(None)).await;
            }
        });
    }

    pub async fn broadcast(this: Arc<Mutex<Self>>, data: Request) {
        let peers: Vec<Arc<Mutex<Peer>>> = {
            let guard = this
                .lock()
                .await;
            guard
                .peers
                .clone()
        };
        for p in peers {
            let sink = {
                let peer = p
                    .lock()
                    .await;
                Arc::clone(&peer.sink)
            };
            let mut sink = sink
                .lock()
                .await;
            let _ = sink
                .send(data.clone())
                .await;
        }
    }
}

struct Peer {
    parent: Weak<Mutex<Network>>,
    sink: Arc<Mutex<SplitSink<Framed<TokioFramed<TcpStream, LengthDelimitedCodec>, Request, Request, Json<Request, Request>>, Request>>>,
}

impl Peer {
    fn new(parent: Weak<Mutex<Network>>, stream: TcpStream) -> Self {
        let framed = Framed::new(TokioFramed::new(stream, LengthDelimitedCodec::new()), Json::default());
        let (sink, stream) = framed.split();
        let sink = Arc::new(Mutex::new(sink));
        let reader = parent.clone();
        spawn(async move {
            Peer::read_loop(reader, stream).await;
        });
        Self {
            parent,
            sink,
        }
    }

    async fn start(&mut self) {
        let mut sink = self
            .sink
            .lock()
            .await;
        let _ = sink
            .send(Request::Ibd(None))
            .await;
    }

    async fn read_loop(parent: Weak<Mutex<Network>>, mut stream: SplitStream<Framed<TokioFramed<TcpStream, LengthDelimitedCodec>, Request, Request, Json<Request, Request>>>) {
        loop {
            while let Some(req) = stream
                .next()
                .await
            {
                if let Ok(msg) = req {
                    Peer::handle(parent.clone(), msg).await;
                }
            }
        }
    }

    async fn handle(parent: Weak<Mutex<Network>>, req: Request) {
        match req {
            Request::Block(mut block) => {
                if block
                    .pow
                    .is_some()
                {
                    if !block.verify_pow() {
                        eprintln!("Rejecting remote block, POW verification failed");
                        return;
                    }
                    println!("Accepting and storing remote block: {:#?}", block);
                    let network = parent
                        .upgrade()
                        .unwrap();
                    let bc = {
                        network
                            .lock()
                            .await
                            .blockchain
                            .clone()
                    };
                    bc.lock()
                        .await
                        .store(block)
                } else {
                    block.calc_set_pow();
                    let network = parent
                        .upgrade()
                        .unwrap();
                    {
                        let self_bc = {
                            network
                                .lock()
                                .await
                                .blockchain
                                .clone()
                        };
                        self_bc
                            .lock()
                            .await
                            .store(block.clone());
                    }
                    Network::broadcast(network, Request::Block(block)).await;
                }
            }
            Request::Ibd(bc) => match bc {
                Some(bc) => {
                    let network = parent
                        .upgrade()
                        .unwrap();
                    let self_bc = {
                        network
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
                        *self_bc = bc;
                    } else if *self_bc != bc {
                        eprintln!("Remote IBD broadcast did not match ours, {:#?} vs. {:#?}", bc, self_bc);
                    }
                }
                None => {
                    let network = parent
                        .upgrade()
                        .unwrap();
                    let bc = {
                        let bc = {
                            network
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
                    Network::broadcast(network, Request::Ibd(Some(bc))).await;
                }
            },
        }
    }
}
