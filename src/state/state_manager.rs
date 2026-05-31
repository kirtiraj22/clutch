use std::collections::HashMap;
use std::sync::Arc;

use sha2::{Digest, Sha256};
use solana_sdk::pubkey::Pubkey;
use tokio::sync::RwLock;
use tracing::{debug, instrument};

use crate::storage::{self, Store};
use crate::types::{ClutchError, L2Account};

#[derive(Clone)]
pub struct StateManager {
    cache: Arc<RwLock<HashMap<String, L2Account>>>,
    store: Store,
    state_root: Arc<RwLock<String>>,
}

impl StateManager {
    pub async fn new(store: Store) -> Result<Self, ClutchError> {
        let state_root = store
            .get::<String>(storage::STATE_ROOT_KEY).await?
            .unwrap_or_else(|| "0".repeat(64));

        Ok(Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            store,
            state_root: Arc::new(RwLock::new(state_root)),
        })
    }


    #[instrument(skip(self), fields(pubkey = %pubkey))]
    pub async fn get_account(&self, pubkey: &Pubkey) -> Option<L2Account> {
        let key_str = pubkey.to_string();

        if let Some(acct) = self.cache.read().await.get(&key_str) {
            debug!("cache hit");
            return Some(acct.clone());
        }

        let db_key = storage::account_key(&key_str);
        if let Ok(Some(acct)) = self.store.get::<L2Account>(&db_key).await {
            debug!("storage hit");
            self.cache.write().await.insert(key_str, acct.clone());
            return Some(acct);
        }

        debug!("miss");
        None
    }

    pub async fn get_nonce(&self, pubkey: &Pubkey) -> u64 {
        self.get_account(pubkey).await.map(|a| a.nonce).unwrap_or(0)
    }

    pub async fn get_state_root(&self) -> String {
        // self.state_root.blocking_read().clone()
        self.state_root.read().await.clone()
    }

    pub async fn get_state_root_async(&self) -> String {
        self.state_root.read().await.clone()
    }

    #[instrument(skip(self, account), fields(pubkey = %pubkey))]
    pub async fn update_account(&self, pubkey: &Pubkey, account: L2Account) -> Result<(), ClutchError> {
        let key_str = pubkey.to_string();

        self.cache.write().await.insert(key_str.clone(), account.clone());
        self.store.put(storage::account_key(&key_str), &account).await?;

        debug!(lamports = account.lamports, nonce = account.nonce, "account updated");

        self.recompute_state_root().await?;
        Ok(())
    }

    async fn recompute_state_root(&self) -> Result<(), ClutchError> {
        let cache = self.cache.read().await;

        let mut entries: Vec<(&String, &L2Account)> = cache.iter().collect();
        entries.sort_by_key(|(k, _)| k.as_str());

        let leaves: Vec<[u8; 32]> = entries
            .iter()
            .map(|(pubkey, acct)| {
                let mut h = Sha256::new();
                h.update(pubkey.as_bytes());
                h.update(acct.lamports.to_le_bytes());
                h.update(acct.nonce.to_le_bytes());
                let data_hash = Sha256::digest(&acct.data);
                h.update(data_hash);
                h.finalize().into()
            })
            .collect();

        let root = if leaves.is_empty() {
            [0u8; 32]
        } else {
            let mut h = Sha256::new();
            for leaf in &leaves {
                h.update(leaf);
            }
            h.finalize().into()
        };

        let root_hex = hex::encode(root);
        *self.state_root.write().await = root_hex.clone();
        self.store.put(storage::STATE_ROOT_KEY, &root_hex).await?;

        debug!(root = %root_hex, accounts = entries.len(), "state root updated");
        Ok(())
    }
}
