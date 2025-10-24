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

    pub fn push(&mut self, data: T) {
        let prev_hash = self
            .data
            .last()
            .map(|p| {
                p.0.clone()
            })
            .unwrap_or_default();
        self.data
            .push((Self::hash_item((&prev_hash, &data)), data));
    }

    // TODO: make try_last that does same as this but returns Err() if it's not the
    // last one and/or refuse to insert
    pub fn insert(&mut self, data: T, base_hash: &[u8]) {
        let pos = self
            .data
            .iter()
            .position(|x| x.0 == *base_hash)
            .unwrap();
        self.data
            .insert(pos + 1, (Self::hash_item((base_hash, &data)), data));

        // everything after that hash is invalidated
        self.rehash(&base_hash);
    }

    pub fn verify(&self, until_hash: &[u8]) -> bool {
        let pos = self
            .data
            .iter()
            .position(|x| x.0 == until_hash)
            .unwrap();

        self.data
            .iter()
            .take(pos + 1)
            .try_fold(Vec::<u8>::new(), |prev, (hash, val)| {
                let expected = Self::hash_item((&prev, val));
                if *hash == expected {
                    Ok(expected)
                } else {
                    Err(())
                }
            })
            .is_ok()
    }

    pub fn rehash(&mut self, from_hash: &[u8]) {
        let pos = self
            .data
            .iter()
            .position(|x| x.0 == *from_hash)
            .unwrap();

        let mut prev = self.data[pos]
            .0
            .clone();
        self.data
            .iter_mut()
            .skip(pos + 1)
            .for_each(|(hash, val)| {
                let expected = Self::hash_item((&prev, val));
                if *hash != expected {
                    *hash = expected.clone();
                }
                prev = expected;
            });
    }

    pub fn hash_item(data: (&[u8], &T)) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(bincode::encode_to_vec(data, bincode::config::standard()).unwrap());
        hasher
            .finalize()
            .to_vec()
    }

    pub fn hash_o_item(data: &(Vec<u8>, T)) -> Vec<u8> {
        Self::hash_item((&data.0, &data.1))
    }
}

impl<T: Encode> Deref for HashChain<T> {
    type Target = Vec<(Vec<u8>, T)>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}
