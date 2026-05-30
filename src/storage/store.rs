use std::sync::Arc;

use rocksdb::{Options, DB};
use serde::{de::DeserializeOwned, Serialize};

use crate::types::ClutchError;

#[derive(Clone)]
pub struct Store {
    db: Arc<DB>,
}

impl Store {
    pub fn open(path: &str) -> Result<Self, ClutchError> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_compression_type(rocksdb::DBCompressionType::Lz4);

        let db = DB::open(&opts, path)?;
        Ok(Self { db: Arc::new(db) })
    }
    pub fn put<V: Serialize>(&self, key: impl AsRef<[u8]>, value: &V) -> Result<(), ClutchError> {
        let bytes = bincode::serialize(value)?;
        self.db.put(key, bytes)?;
        Ok(())
    }

    pub fn get<V: DeserializeOwned>(&self, key: impl AsRef<[u8]>) -> Result<Option<V>, ClutchError> {
        match self.db.get(key)? {
            Some(bytes) => Ok(Some(bincode::deserialize(&bytes)?)),
            None => Ok(None),
        }
    }
    pub fn delete(&self, key: impl AsRef<[u8]>) -> Result<(), ClutchError> {
        self.db.delete(key)?;
        Ok(())
    }

    pub fn scan_prefix<V: DeserializeOwned>(&self, prefix: &[u8]) -> Vec<V> {
        self.db
            .iterator(rocksdb::IteratorMode::From(prefix, rocksdb::Direction::Forward))
            .filter_map(|r| r.ok())
            .take_while(|(k, _)| k.starts_with(prefix))
            .filter_map(|(_, v)| bincode::deserialize::<V>(&v).ok())
            .collect()
    }
}


pub fn account_key(pubkey: &str) -> Vec<u8> {
    format!("acct:{pubkey}").into_bytes()
}

pub fn block_key(number: u64) -> Vec<u8> {
    format!("block:{:016x}", number).into_bytes()
}

pub fn batch_key(number: u64) -> Vec<u8> {
    format!("batch:{:016x}", number).into_bytes()
}

pub fn receipt_key(tx_id: &str) -> Vec<u8> {
    format!("receipt:{tx_id}").into_bytes()
}

pub fn nonce_key(pubkey: &str) -> Vec<u8> {
    format!("nonce:{pubkey}").into_bytes()
}

pub const STATE_ROOT_KEY: &[u8] = b"meta:state_root";
pub const BLOCK_HEIGHT_KEY: &[u8] = b"meta:block_height";
pub const BATCH_NUMBER_KEY: &[u8] = b"meta:batch_number";
