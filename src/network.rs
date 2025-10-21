use std::sync::{Arc, Weak};

use serde::{Deserialize, Serialize};

use futures::{SinkExt, StreamExt};
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
pub enum Request {
    Block(Block),
    Ibd(Option<BlockChain>),
}

pub struct Network {
    this: Weak<Mutex<Self>>,
    me: Client,
    listener: Arc<TcpListener>,
    peers: Vec<Arc<Mutex<Peer>>>,
    blockchain: Arc<Mutex<BlockChain>>,
}

impl Network {
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
        }));
        this.lock()
            .await
            .this = Arc::downgrade(&this);
        this
    }

    async fn try_peer(&mut self) {
        let peer = Arc::new(Mutex::new(Peer::new(
            self.this
                .clone(),
            TcpStream::connect("192.168.143.90:6767")
                .await
                .unwrap(),
        )));
        self.peers
            .push(peer.clone());

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
                    let mut net = parent
                        .lock()
                        .await;
                    net.peers
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
            if let Some(parent) = this.upgrade() {
                let mut net = parent
                    .lock()
                    .await;
                net.try_peer()
                    .await;
            }
        });
    }

    pub async fn broadcast(&mut self, data: Request) {
        for p in &self.peers {
            p.lock()
                .await
                .framed
                .send(data.clone())
                .await
                .unwrap();
        }
    }
}

struct Peer {
    parent: Weak<Mutex<Network>>,
    framed: Framed<TokioFramed<TcpStream, LengthDelimitedCodec>, Request, Request, Json<Request, Request>>,
}

impl Peer {
    fn new(parent: Weak<Mutex<Network>>, stream: TcpStream) -> Self {
        eprintln!("a");
        Self {
            parent,
            framed: Framed::new(TokioFramed::new(stream, LengthDelimitedCodec::new()), Json::default()),
        }
    }

    async fn start(&mut self) {
        loop {
            while let Some(req) = self
                .framed
                .next()
                .await
            {
                eprintln!("a");
                self.handle(req.unwrap())
                    .await;
            }
        }
    }

    async fn handle(&mut self, req: Request) {
        match req {
            Request::Block(mut block) => match block.pow {
                Some(_) => self
                    .parent
                    .upgrade()
                    .unwrap()
                    .lock()
                    .await
                    .blockchain
                    .lock()
                    .await
                    .store(block),
                None => {
                    block.calc_set_pow();
                    let parent = self
                        .parent
                        .upgrade()
                        .unwrap();
                    let mut net = parent
                        .lock()
                        .await;
                    net.broadcast(Request::Block(block))
                        .await;
                }
            },
            Request::Ibd(bc) => match bc {
                Some(_bc) => {
                    let parent = self
                        .parent
                        .upgrade()
                        .unwrap();
                    let net = parent
                        .lock()
                        .await;
                    let mut _bc_lock = net
                        .blockchain
                        .lock()
                        .await;
                }
                None => {
                    let parent = self
                        .parent
                        .upgrade()
                        .unwrap();
                    let mut net = parent
                        .lock()
                        .await;
                    let bc = {
                        let guard = net
                            .blockchain
                            .lock()
                            .await;
                        guard.clone()
                    };
                    net.broadcast(Request::Ibd(Some(bc)))
                        .await;
                }
            },
        }
    }
}
