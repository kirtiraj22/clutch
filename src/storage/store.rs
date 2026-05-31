use std::{
    collections::HashMap,
    sync::Arc,
};

use serde::{de::DeserializeOwned, Serialize};
use tokio::sync::RwLock;

use crate::types::ClutchError;

#[derive(Clone)]
pub struct Store {
    inner: Arc<RwLock<HashMap<Vec<u8>, Vec<u8>>>>,
}

impl Store {
    pub fn open(_path: &str) -> Result<Self, ClutchError> {
        Ok(Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn put<V: Serialize>(
        &self,
        key: impl AsRef<[u8]>,
        value: &V,
    ) -> Result<(), ClutchError> {
        let bytes = bincode::serialize(value)?;
        self.inner
            .write()
            .await
            .insert(key.as_ref().to_vec(), bytes);

        Ok(())
    }

    pub async fn get<V: DeserializeOwned>(
        &self,
        key: impl AsRef<[u8]>,
    ) -> Result<Option<V>, ClutchError> {
        let map = self.inner.read().await;

        match map.get(key.as_ref()) {
            Some(bytes) => Ok(Some(bincode::deserialize(bytes)?)),
            None => Ok(None),
        }
    }

    pub async fn delete(
        &self,
        key: impl AsRef<[u8]>,
    ) -> Result<(), ClutchError> {
        self.inner.write().await.remove(key.as_ref());
        Ok(())
    }

    pub async fn scan_prefix<V: DeserializeOwned>(
        &self,
        prefix: &[u8],
    ) -> Vec<V> {
        let map = self.inner.read().await;

        map.iter()
            .filter(|(k, _)| k.starts_with(prefix))
            .filter_map(|(_, v)| bincode::deserialize::<V>(v).ok())
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