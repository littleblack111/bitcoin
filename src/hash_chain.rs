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
            .push((Self::with_hash((&prev_hash, &data)), data));
    }

    // TODO: make try_last that does same as this but returns Err() if it's not the
    // last one and/or refuse to insert
    fn insert(&mut self, data: T, base_hash: &Vec<u8>) {
        let pos = self
            .data
            .iter()
            .position(|x| x.0 == *base_hash)
            .unwrap();
        self.data
            .insert(pos + 1, (Self::with_hash((base_hash, &data)), data));

        // everything after that hash is invalidated
        self.rehash(&base_hash);
    }

    fn verify(&self, until_hash: &[u8]) -> bool {
        let pos = self
            .data
            .iter()
            .position(|x| x.0 == until_hash)
            .unwrap();
        self.data[pos..=1]
            .iter()
            .enumerate()
            .all(|(prev_hash, (pred_prev_hash, _))| {
                let prev_hash = Self::with_hash({
                    let prev_hash = &self.data[prev_hash - 1];
                    (&prev_hash.0, &prev_hash.1)
                });
                prev_hash == *pred_prev_hash
            })
    }

    fn rehash(&mut self, from_hash: &[u8]) {
        let pos = self
            .data
            .iter()
            .position(|x| x.0 == *from_hash)
            .unwrap();

        let prev_hashes: Vec<_> = (pos..self
            .data
            .len())
            .map(|i| {
                Self::with_hash({
                    let prev_hash = &self.data[i - 1];
                    (&prev_hash.0, &prev_hash.1)
                })
            })
            .collect();

        self.data[pos..]
            .iter_mut()
            .enumerate()
            .map(|(idx, item)| (item, prev_hashes[idx].clone()))
            .for_each(|((pred_prev_hash, _), prev_hash)| {
                if prev_hash != *pred_prev_hash {
                    *pred_prev_hash = prev_hash;
                }
            });
    }

    fn with_hash(data: (&Vec<u8>, &T)) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(bincode::encode_to_vec(data, bincode::config::standard()).unwrap());
        hasher
            .finalize()
            .to_vec()
    }

    fn hash_last(&self) -> Vec<u8> {
        Self::with_hash({
            let last = self
                .data
                .iter()
                .last()
                .unwrap();
            (&last.0, &last.1)
        })
    }
}

impl<T: Encode> Deref for HashChain<T> {
    type Target = Vec<(Vec<u8>, T)>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}
