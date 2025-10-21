use bincode::Encode;
use sha2::{Digest, Sha256, digest::DynDigest};
use std::{
    hash::{Hash, Hasher},
    ops::Deref,
};

use serde::{Deserialize, Serialize};

use crate::{ZERO_PREFIX_AMOUNT, transaction::Transaction};

pub trait CryptoDigest {
    fn digest(&self, state: &mut impl DynDigest);
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Encode)]
pub struct Block {
    pub prev_hash: Vec<u8>,
    pub trans: Transaction,
    pub pow: Vec<u8>,
}

impl Block {
    fn new(prev_hash: Option<&[u8]>, trans: Transaction) -> Self {
        Self {
            prev_hash: prev_hash
                .unwrap_or(&[])
                .to_vec(), // special blocks
            trans,
            pow: Vec::default(),
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

    fn calc_pow(&self) -> Vec<u8> {
        for i in 0.. {
            // reset every loop
            let mut hasher = Sha256::new();
            Digest::update(&mut hasher, bincode::encode_to_vec((&self.prev_hash, &self.trans, i), bincode::config::standard()).unwrap());
            let hashed = hasher.finalize();
            if Self::pref_zeros(&hashed).is_ok() {
                return hashed.to_vec();
            }
        }
        unreachable!()
    }

    pub fn verify_pow(&mut self) -> bool {
        let mut hasher = Sha256::new();
        Digest::update(&mut hasher, bincode::encode_to_vec(&*self, bincode::config::standard()).unwrap());
        Self::pref_zeros(&hasher.finalize()).is_ok()
    }

    pub fn calc_set_pow(&mut self) {
        self.pow = self.calc_pow();
    }
}

impl CryptoDigest for Block {
    fn digest(&self, state: &mut impl DynDigest) {
        state.update(&bincode::encode_to_vec((&self.prev_hash, &self.trans, &self.pow), bincode::config::standard()).unwrap());
    }
}

#[derive(Default, Deserialize, Serialize, Clone, PartialEq)]
pub struct BlockChain {
    pub blocks: Vec<Block>,
}

impl BlockChain {
    pub fn new(blocks: Vec<Block>) -> Self {
        Self {
            blocks,
        }
    }

    pub fn new_block(&self, trans: Transaction) -> Block {
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
        Block::new(prev_hash, trans)
    }

    pub fn store(&mut self, block: Block) {
        // TODO: verify blocks
        self.blocks
            .push(block)
    }
}

impl Deref for BlockChain {
    type Target = Vec<Block>;

    fn deref(&self) -> &Self::Target {
        &self.blocks
    }
}
