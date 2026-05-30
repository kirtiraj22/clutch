use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionKind {
    Transfer { to: Pubkey, lamports: u64 },
    Mint { to: Pubkey, lamports: u64 },
    Burn { lamports: u64 },
    CustomInstruction { program_id: Pubkey, data: Vec<u8> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TxStatus {
    Pending,
    Executed,
    Failed(String),
    Finalized,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2Transaction {
    pub id: String,
    pub from: Pubkey,
    pub kind: TransactionKind,
    pub nonce: u64,pub received_at: DateTime<Utc>,
    pub raw: Vec<u8>,

    pub status: TxStatus,
}

impl L2Transaction {
    pub fn new(id: String, from: Pubkey, kind: TransactionKind, nonce: u64, raw: Vec<u8>) -> Self {
        Self {
            id,
            from,
            kind,
            nonce,
            received_at: Utc::now(),
            raw,
            status: TxStatus::Pending,
        }
    }
}
