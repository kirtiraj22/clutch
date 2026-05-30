use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::L2Block;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BatchStatus {
    Pending,
    Submitted { l1_signature: String },
    Confirmed { l1_slot: u64 },
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchMeta {
    pub batch_number: u64,
    pub first_block: u64,
    pub last_block: u64,
    pub total_txs: usize,
    pub sealed_at: DateTime<Utc>,
    pub compressed_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2Batch {
    pub meta: BatchMeta,
    pub blocks: Vec<L2Block>,
    pub status: BatchStatus,
}

impl L2Batch {
    pub fn new(batch_number: u64, blocks: Vec<L2Block>, compressed_bytes: usize) -> Self {
        let first_block = blocks.first().map(|b| b.header.number).unwrap_or(0);
        let last_block = blocks.last().map(|b| b.header.number).unwrap_or(0);
        let total_txs = blocks.iter().map(|b| b.tx_count()).sum();

        Self {
            meta: BatchMeta {
                batch_number,
                first_block,
                last_block,
                total_txs,
                sealed_at: chrono::Utc::now(),
                compressed_bytes,
            },
            blocks,
            status: BatchStatus::Pending,
        }
    }
}
