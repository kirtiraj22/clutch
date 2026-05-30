use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClutchError {
    #[error("Invalid transaction signature")]
    InvalidSignature,

    #[error("Insufficient funds: need {needed}, have {available}")]
    InsufficientFunds { needed: u64, available: u64 },

    #[error("Invalid nonce: expected {expected}, got {got}")]
    InvalidNonce { expected: u64, got: u64 },

    #[error("Unknown transaction kind: {0}")]
    UnknownInstruction(String),

    #[error("Transaction not found: {0}")]
    TxNotFound(String),

    #[error("Account not found: {0}")]
    AccountNotFound(String),

    #[error("State root computation failed: {0}")]
    StateRootError(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Batch channel closed")]
    BatchChannelClosed,

    #[error("L1 submission failed: {0}")]
    L1SubmissionFailed(String),

    #[error("Invalid public key: {0}")]
    InvalidPubkey(String),

    #[error("Invalid transaction encoding: {0}")]
    InvalidEncoding(String),
}

impl From<rocksdb::Error> for ClutchError {
    fn from(e: rocksdb::Error) -> Self {
        ClutchError::Storage(e.to_string())
    }
}

impl From<bincode::Error> for ClutchError {
    fn from(e: bincode::Error) -> Self {
        ClutchError::Serialization(e.to_string())
    }
}
