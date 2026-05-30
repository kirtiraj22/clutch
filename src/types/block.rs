use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::L2Transaction;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    pub number: u64,
    pub parent_hash: String,
    pub state_root: String,
    pub tx_root: String,
    pub timestamp: DateTime<Utc>,
    pub sequencer: String,
}

impl BlockHeader {
    pub fn hash(&self) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(self.number.to_le_bytes());
        hasher.update(self.parent_hash.as_bytes());
        hasher.update(self.state_root.as_bytes());
        hasher.update(self.tx_root.as_bytes());
        hasher.update(self.timestamp.timestamp().to_le_bytes());
        hex::encode(hasher.finalize())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2Block {
    pub header: BlockHeader,
    pub transactions: Vec<L2Transaction>,
    pub hash: String,
}

impl L2Block {
    pub fn new(header: BlockHeader, transactions: Vec<L2Transaction>) -> Self {
        let hash = header.hash();
        Self { header, transactions, hash }
    }

    pub fn tx_count(&self) -> usize {
        self.transactions.len()
    }
}
