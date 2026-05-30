use std::sync::Arc;

use jsonrpsee::{
    core::{async_trait, RpcResult},
    proc_macros::rpc,
    types::ErrorObjectOwned,
};
use serde_json::Value;
use solana_sdk::{pubkey::Pubkey, transaction::Transaction};
use tokio::sync::RwLock;
use tracing::{info, instrument};

use crate::{
    mempool::Mempool,
    rpc::decode,
    sequencer::Sequencer,
    state::StateManager,
    storage::{self, Store},
    types::TxReceipt,
};


#[rpc(server)]
pub trait RollupRpc {
    #[method(name = "getAccountInfo")]
    async fn get_account_info(&self, pubkey: String, config: Option<Value>) -> RpcResult<Value>;

    #[method(name = "getBalance")]
    async fn get_balance(&self, pubkey: String, config: Option<Value>) -> RpcResult<u64>;

    #[method(name = "sendTransaction")]
    async fn send_transaction(&self, encoded: String, config: Option<Value>) -> RpcResult<String>;

    #[method(name = "getLatestBlockhash")]
    async fn get_latest_blockhash(&self, config: Option<Value>) -> RpcResult<Value>;

    #[method(name = "simulateTransaction")]
    async fn simulate_transaction(&self, encoded: String, config: Option<Value>) -> RpcResult<Value>;

    #[method(name = "getTransaction")]
    async fn get_transaction(&self, signature: String, config: Option<Value>) -> RpcResult<Option<Value>>;

    #[method(name = "clutch_getChainStatus")]
    async fn get_chain_status(&self) -> RpcResult<Value>;

    #[method(name = "clutch_getLatestBlock")]
    async fn get_latest_block(&self) -> RpcResult<Option<Value>>;

    #[method(name = "clutch_getLatestBatch")]
    async fn get_latest_batch(&self) -> RpcResult<Option<Value>>;

    #[method(name = "clutch_getRecentBlocks")]
    async fn get_recent_blocks(&self, limit: Option<usize>) -> RpcResult<Value>;

    #[method(name = "clutch_getRecentBatches")]
    async fn get_recent_batches(&self, limit: Option<usize>) -> RpcResult<Value>;

    #[method(name = "clutch_getPendingTxs")]
    async fn get_pending_txs(&self) -> RpcResult<Value>;

    #[method(name = "clutch_getTransactionReceipt")]
    async fn get_transaction_receipt(&self, tx_id: String) -> RpcResult<Option<Value>>;
}

pub struct RollupRpcImpl {
    state: Arc<StateManager>,
    mempool: Arc<Mempool>,
    sequencer: Arc<Sequencer>,
    store: Store,
    receipts: Arc<RwLock<Vec<TxReceipt>>>,
}

impl RollupRpcImpl {
    pub fn new(
        state: Arc<StateManager>,
        mempool: Arc<Mempool>,
        sequencer: Arc<Sequencer>,
        store: Store,
        receipts: Arc<RwLock<Vec<TxReceipt>>>,
    ) -> Self {
        Self { state, mempool, sequencer, store, receipts }
    }
}


fn rpc_err(code: i32, msg: &str, detail: impl ToString) -> ErrorObjectOwned {
    ErrorObjectOwned::owned(code, msg, Some(detail.to_string()))
}


#[async_trait]
impl RollupRpcServer for RollupRpcImpl {

    #[instrument(skip(self), fields(pubkey = %pubkey))]
    async fn get_account_info(&self, pubkey: String, _config: Option<Value>) -> RpcResult<Value> {
        let key: Pubkey = pubkey
            .parse()
            .map_err(|e| rpc_err(-32602, "Invalid pubkey", e))?;

        match self.state.get_account(&key).await {
            Some(acct) => Ok(serde_json::json!({
                "value": {
                    "lamports": acct.lamports,
                    "owner": acct.owner.to_string(),
                    "executable": acct.executable,
                    "nonce": acct.nonce,
                    "data": [bs58::encode(&acct.data).into_string(), "base58"],
                }
            })),
            None => Ok(serde_json::json!({ "value": null })),
        }
    }

    async fn get_balance(&self, pubkey: String, _config: Option<Value>) -> RpcResult<u64> {
        let key: Pubkey = pubkey
            .parse()
            .map_err(|e| rpc_err(-32602, "Invalid pubkey", e))?;
        Ok(self.state.get_account(&key).await.map(|a| a.lamports).unwrap_or(0))
    }

    async fn send_transaction(&self, encoded: String, _config: Option<Value>) -> RpcResult<String> {
        let raw_bytes = bs58::decode(&encoded)
            .into_vec()
            .map_err(|e| rpc_err(-32602, "Invalid base58 encoding", e))?;

        let solana_tx: Transaction = bincode::deserialize(&raw_bytes)
            .map_err(|e| rpc_err(-32602, "Invalid transaction format", e))?;

        let l2_tx = decode::decode_transaction(&solana_tx, raw_bytes)
            .map_err(|e| rpc_err(-32000, "Transaction decode failed", e))?;

        let tx_id = l2_tx.id.clone();

        if !self.mempool.accept(l2_tx).await {
            return Err(rpc_err(-32000, "Transaction rejected", "duplicate or mempool full"));
        }

        info!(tx_id = %tx_id, "transaction accepted into mempool");
        Ok(tx_id)
    }

    async fn get_latest_blockhash(&self, _config: Option<Value>) -> RpcResult<Value> {
        let root = self.state.get_state_root_async().await;
        
        let blockhash = format!("{}{}", &root[..32], "1111111111111111111111111111111");
        let height = self
            .store
            .get::<u64>(storage::BLOCK_HEIGHT_KEY)
            .ok()
            .flatten()
            .unwrap_or(0);

        Ok(serde_json::json!({
            "value": {
                "blockhash": blockhash,
                "lastValidBlockHeight": height + 150
            }
        }))
    }

    async fn simulate_transaction(&self, encoded: String, _config: Option<Value>) -> RpcResult<Value> {
        let raw_bytes = bs58::decode(&encoded)
            .into_vec()
            .map_err(|e| rpc_err(-32602, "Invalid base58 encoding", e))?;

        let solana_tx: Transaction = bincode::deserialize(&raw_bytes)
            .map_err(|e| rpc_err(-32602, "Invalid transaction format", e))?;

        let _l2_tx = decode::decode_transaction(&solana_tx, raw_bytes)
            .map_err(|e| rpc_err(-32000, "Transaction decode failed", e))?;

        Ok(serde_json::json!({
            "value": {
                "err": null,
                "logs": ["Program log: simulated execution OK"],
                "unitsConsumed": 5000
            }
        }))
    }

    async fn get_transaction(&self, signature: String, _config: Option<Value>) -> RpcResult<Option<Value>> {
        // Delegate to the receipt store.
        match self.store.get::<TxReceipt>(storage::receipt_key(&signature)) {
            Ok(Some(receipt)) => Ok(Some(serde_json::to_value(receipt).unwrap_or(Value::Null))),
            _ => Ok(None),
        }
    }

    async fn get_chain_status(&self) -> RpcResult<Value> {
        let block_height = self
            .store
            .get::<u64>(storage::BLOCK_HEIGHT_KEY)
            .ok()
            .flatten()
            .unwrap_or(0);
        let batch_number = self
            .store
            .get::<u64>(storage::BATCH_NUMBER_KEY)
            .ok()
            .flatten()
            .unwrap_or(0);
        let state_root = self.state.get_state_root_async().await;
        let pending = self.mempool.pending_count().await;

        Ok(serde_json::json!({
            "chain": "clutch-devnet",
            "blockHeight": block_height,
            "batchNumber": batch_number,
            "stateRoot": state_root,
            "pendingTxs": pending,
        }))
    }

    async fn get_latest_block(&self) -> RpcResult<Option<Value>> {
        match self.sequencer.latest_block().await {
            Some(block) => Ok(Some(serde_json::to_value(block).unwrap_or(Value::Null))),
            None => Ok(None),
        }
    }

    async fn get_latest_batch(&self) -> RpcResult<Option<Value>> {
        match self.sequencer.latest_batch().await {
            Some(batch) => Ok(Some(serde_json::to_value(batch).unwrap_or(Value::Null))),
            None => Ok(None),
        }
    }

    async fn get_recent_blocks(&self, limit: Option<usize>) -> RpcResult<Value> {
        let blocks = self.sequencer.recent_blocks(limit.unwrap_or(10)).await;
        Ok(serde_json::to_value(blocks).unwrap_or(Value::Array(vec![])))
    }

    async fn get_recent_batches(&self, limit: Option<usize>) -> RpcResult<Value> {
        let batches = self.sequencer.recent_batches(limit.unwrap_or(10)).await;
        Ok(serde_json::to_value(batches).unwrap_or(Value::Array(vec![])))
    }

    async fn get_pending_txs(&self) -> RpcResult<Value> {
        let ids = self.mempool.pending_ids().await;
        Ok(serde_json::json!({
            "count": ids.len(),
            "ids": ids,
        }))
    }

    async fn get_transaction_receipt(&self, tx_id: String) -> RpcResult<Option<Value>> {
        
        {
            let receipts = self.receipts.read().await;
            if let Some(r) = receipts.iter().find(|r| r.tx_id == tx_id) {
                return Ok(Some(serde_json::to_value(r).unwrap_or(Value::Null)));
            }
        }

        match self.store.get::<TxReceipt>(storage::receipt_key(&tx_id)) {
            Ok(Some(r)) => Ok(Some(serde_json::to_value(r).unwrap_or(Value::Null))),
            _ => Ok(None),
        }
    }
}
