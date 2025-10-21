use std::{
    hash::{DefaultHasher, Hash, Hasher},
    ops::Deref,
};

use serde::{Deserialize, Serialize};

use crate::{ZERO_PREFIX_AMOUNT, transaction::Transaction};

#[derive(Deserialize, Serialize, Clone, Copy, PartialEq)]
pub struct Block {
    pub prev_hash: u64,
    pub trans: Transaction,
    pub pow: Option<u64>,
}

impl Block {
    fn new(prev_hash: Option<u64>, trans: Transaction) -> Self {
        Self {
            prev_hash: prev_hash.unwrap_or(0), // special blocks
            trans,
            pow: None,
        }
    }

    fn calc_pow(&self) -> u64 {
        for i in 0.. {
            // reset every loop
            let mut hasher = DefaultHasher::new();
            (&self.prev_hash, &self.trans, i).hash(&mut hasher);
            let hashed = hasher.finish();
            if hashed
                .to_string()
                .starts_with(
                    &"0".repeat(ZERO_PREFIX_AMOUNT)
                        .to_string(),
                )
            {
                return hashed;
            }
        }
        unreachable!()
    }

    pub fn verify_pow(&mut self, pow: u64) -> bool {
        let mut hasher = DefaultHasher::new();
        (&self.prev_hash, &self.trans, pow).hash(&mut hasher);
        hasher
            .finish()
            .to_string()
            .to_string()
            .starts_with(
                &"0".repeat(ZERO_PREFIX_AMOUNT)
                    .to_string(),
            )
    }

    pub fn calc_set_pow(&mut self) {
        self.pow = Some(self.calc_pow());
    }
}

impl Hash for Block {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.prev_hash
            .hash(state);
        self.trans
            .hash(state);
        if self
            .pow
            .is_none()
        {
            Self::calc_pow(self).hash(state);
        } else {
            self.pow
                .hash(state);
        }
    }
}

#[derive(Default, Deserialize, Serialize, Clone, PartialEq)]
pub struct BlockChain {
    blocks: Vec<Block>,
}

impl BlockChain {
    pub fn new(blocks: Vec<Block>) -> Self {
        Self {
            blocks,
        }
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
