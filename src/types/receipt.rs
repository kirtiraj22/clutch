use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxReceipt {
    pub tx_id: String,
    pub block_number: u64,
    pub block_hash: String,
    pub tx_index: usize,
    pub success: bool,
    pub error: Option<String>,
    pub gas_used: u64,
    pub executed_at: DateTime<Utc>,
    pub logs: Vec<String>,
}

impl TxReceipt {
    pub fn success(tx_id: String, block_number: u64, block_hash: String, tx_index: usize, logs: Vec<String>) -> Self {
        Self {
            tx_id,
            block_number,
            block_hash,
            tx_index,
            success: true,
            error: None,
            gas_used: 5000, 
            executed_at: chrono::Utc::now(),
            logs,
        }
    }

    pub fn failure(tx_id: String, block_number: u64, block_hash: String, tx_index: usize, reason: String) -> Self {
        Self {
            tx_id,
            block_number,
            block_hash,
            tx_index,
            success: false,
            error: Some(reason),
            gas_used: 0,
            executed_at: chrono::Utc::now(),
            logs: vec![],
        }
    }
}
