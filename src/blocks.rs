use bincode::Encode;
use sha2::{Digest, Sha256, digest::DynDigest};
use std::ops::Deref;

use serde::{Deserialize, Serialize};

use crate::{ZERO_PREFIX_AMOUNT, transaction::Transaction};

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::mpsc;
use tokio::task;

pub trait CryptoDigest {
    fn digest(&self, state: &mut impl DynDigest);
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Encode)]
pub struct Block {
    pub prev_hash: Vec<u8>,
    pub tx: Transaction,
    pub pow: Option<u64>,
}

impl Block {
    fn new(prev_hash: Option<&[u8]>, tx: Transaction) -> Self {
        Self {
            prev_hash: prev_hash
                .unwrap_or(&[])
                .to_vec(), // special blocks
            tx,
            pow: None,
        }
    }

    fn pref_zeros(hashed: &[u8]) -> Result<usize, usize> {
        hashed
            .iter()
            .try_fold(0, |nonce, x| {
                if nonce >= ZERO_PREFIX_AMOUNT {
                    Ok(nonce)
                } else if *x == 0 {
                    Ok(nonce + 1)
                } else {
                    Err(nonce)
                }
            })
    }

    pub async fn calc_pow(&self) -> u64 {
        let threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        let found = Arc::new(AtomicBool::new(false));
        let (ptx, mut rx) = mpsc::unbounded_channel::<u64>();
        let prev_hash = Arc::new(
            self.prev_hash
                .clone(),
        );
        let tx = self.tx;
        let mut handles = Vec::with_capacity(threads);
        for t in 0..threads {
            let found = Arc::clone(&found);
            let ptx = ptx.clone();
            let prev_hash = Arc::clone(&prev_hash);
            let start = t as u64;
            handles.push(task::spawn_blocking(move || {
                let mut i = start;
                while !found.load(Ordering::Relaxed) {
                    let mut hasher = Sha256::new();
                    Digest::update(&mut hasher, bincode::encode_to_vec((&*prev_hash, &tx, i), bincode::config::standard()).unwrap());
                    let hashed = hasher.finalize();
                    if Block::pref_zeros(&hashed).is_ok() {
                        if !found.swap(true, Ordering::Relaxed) {
                            let _ = ptx.send(i);
                        }
                        break;
                    }
                    println!("{i}");
                    i = i.wrapping_add(threads as u64);
                }
            }));
        }

        drop(ptx);

        let pow_nonce = rx
            .recv()
            .await
            .expect("miner dropped without sending");
        for h in handles {
            _ = h.await;
        }
        pow_nonce
    }

    pub async fn calc_set_pow(&mut self) {
        self.pow = Some(
            self.calc_pow()
                .await,
        );
    }

    pub fn verify_pow(&self) -> bool {
        if let Some(nonce) = self.pow {
            let mut hasher = Sha256::new();
            Digest::update(&mut hasher, bincode::encode_to_vec((&self.prev_hash, &self.tx, nonce), bincode::config::standard()).unwrap());
            Self::pref_zeros(&hasher.finalize()).is_ok()
        } else {
            false
        }
    }
}

impl CryptoDigest for Block {
    fn digest(&self, state: &mut impl DynDigest) {
        state.update(&bincode::encode_to_vec((&self.prev_hash, &self.tx, &self.pow), bincode::config::standard()).unwrap());
    }
}

#[derive(Debug, Default, Deserialize, Serialize, Clone, PartialEq)]
pub struct BlockChain {
    pub blocks: Vec<Block>,
}

impl BlockChain {
    pub fn new(blocks: Vec<Block>) -> Self {
        Self {
            blocks,
        }
    }

    pub fn new_block(&self, tx: Transaction) -> Block {
        let mut hasher = Sha256::new();
        let prev = self
            .blocks
            .last();
        let prev_hash = if let Some(b) = prev {
            b.digest(&mut hasher);
            Some(&*hasher.finalize())
        } else {
            None
        };
        Block::new(prev_hash, tx)
    }

    pub fn store(&mut self, block: Block) {
        // TODO: check config
        if block.verify_pow() {
            self.blocks
                .push(block)
        } else {
            eprint!("Err: Failed to store unverified block")
        }
    }
}

impl Deref for BlockChain {
    type Target = Vec<Block>;

    fn deref(&self) -> &Self::Target {
        &self.blocks
    }
}
