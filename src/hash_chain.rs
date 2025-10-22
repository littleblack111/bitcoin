use std::ops::Deref;

use bincode::Encode;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, PartialEq, Deserialize, Serialize, Clone)]
pub struct HashChain<T: Encode> {
    data: Vec<(Vec<u8>, T)>, // Vector of (hash, value) pairs
}

impl<T: Encode> Default for HashChain<T> {
    fn default() -> Self {
        Self {
            data: Default::default(),
        }
    }
}

impl<T: Encode> HashChain<T> {
    pub fn new(data: Vec<T>) -> Self {
        let mut this = Self::default();
        for i in data {
            this.push(i);
        }
        this
    }

    fn push(&mut self, data: T) {
        let prev_hash = self
            .data
            .last()
            .map(|p| {
                p.0.clone()
            })
            .unwrap_or_default();
        self.data
            .push((Self::hash(&prev_hash, &data), data));
    }

    fn insert(&mut self, data: T, base_hash: &[u8]) {
        let pos = self
            .data
            .iter()
            .position(|x| x.0 == base_hash)
            .unwrap();
        self.data
            .insert(pos, (Self::hash(base_hash, &data), data));
    }

    fn verify(&self, from_hash: &[u8]) -> bool {
        let i = self
            .data
            .iter()
            .position(|x| x.0 == from_hash)
            .unwrap();
        (i..self
            .data
            .len())
            .all(|i| {
                Self::hash(
                    if i == 0 {
                        &[]
                    } else {
                        &self.data[i - 1].0
                    },
                    &self.data[i].1,
                ) == self.data[i].0
            })
    }

    fn hash(prev_hash: &[u8], data: &T) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(prev_hash);
        hasher.update(bincode::encode_to_vec(data, bincode::config::standard()).unwrap());
        hasher
            .finalize()
            .to_vec()
    }
}

impl<T: Encode> Deref for HashChain<T> {
    type Target = Vec<(Vec<u8>, T)>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}
