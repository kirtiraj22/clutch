use std::collections::{HashSet, VecDeque};
use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::types::L2Transaction;

const MAX_PENDING: usize = 10_000;

#[derive(Clone)]
pub struct Mempool {
    queue: Arc<RwLock<VecDeque<L2Transaction>>>,
    seen: Arc<RwLock<HashSet<String>>>,
}

impl Mempool {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(RwLock::new(VecDeque::new())),
            seen: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    pub async fn accept(&self, tx: L2Transaction) -> bool {
        let mut seen = self.seen.write().await;
        if seen.contains(&tx.id) {
            warn!(tx_id = %tx.id, "duplicate transaction rejected");
            return false;
        }

        let mut queue = self.queue.write().await;
        if queue.len() >= MAX_PENDING {
            warn!("mempool full — dropping transaction {}", tx.id);
            return false;
        }

        seen.insert(tx.id.clone());
        queue.push_back(tx);
        debug!("mempool depth = {}", queue.len());
        true
    }

    pub async fn drain(&self, limit: usize) -> Vec<L2Transaction> {
        let mut queue = self.queue.write().await;
        let take = limit.min(queue.len());
        let drained: Vec<L2Transaction> = queue.drain(..take).collect();
        if !drained.is_empty() {
            info!(count = drained.len(), "drained mempool");
        }
        drained
    }

    pub async fn pending_count(&self) -> usize {
        self.queue.read().await.len()
    }

    pub async fn pending_ids(&self) -> Vec<String> {
        self.queue.read().await.iter().map(|tx| tx.id.clone()).collect()
    }
}

impl Default for Mempool {
    fn default() -> Self {
        Self::new()
    }
}
