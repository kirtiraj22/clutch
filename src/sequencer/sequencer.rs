use std::sync::Arc;

use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, Duration};
use tracing::{error, info};

use crate::mempool::Mempool;
use crate::runtime::Runtime;
use crate::state::StateManager;
use crate::storage::{self, Store};
use crate::types::{BlockHeader, L2Batch, L2Block, L2Transaction, TxReceipt};

#[derive(Debug, Clone)]
pub struct SequencerConfig {
    pub block_interval_secs: u64,
    pub max_txs_per_block: usize,
    pub blocks_per_batch: usize,
    pub sequencer_id: String,
}

impl Default for SequencerConfig {
    fn default() -> Self {
        Self {
            block_interval_secs: 2,
            max_txs_per_block: 100,
            blocks_per_batch: 5,
            sequencer_id: "clutch-sequencer-0".to_string(),
        }
    }
}

#[derive(Clone)]
pub struct Sequencer {
    config: SequencerConfig,
    mempool: Arc<Mempool>,
    runtime: Arc<Runtime>,
    state: Arc<StateManager>,
    store: Store,
    pending_blocks: Arc<RwLock<Vec<L2Block>>>,
    latest_block: Arc<RwLock<Option<L2Block>>>,
    latest_batch: Arc<RwLock<Option<L2Batch>>>,
    batch_tx: mpsc::Sender<L2Batch>,
    receipts: Arc<RwLock<Vec<TxReceipt>>>,
}

impl Sequencer {
    pub fn new(
        config: SequencerConfig,
        mempool: Arc<Mempool>,
        runtime: Arc<Runtime>,
        state: Arc<StateManager>,
        store: Store,
        receipts: Arc<RwLock<Vec<TxReceipt>>>,
    ) -> (Self, mpsc::Receiver<L2Batch>) {
        let (batch_tx, batch_rx) = mpsc::channel(32);

        let seq = Self {
            config,
            mempool,
            runtime,
            state,
            store,
            pending_blocks: Arc::new(RwLock::new(Vec::new())),
            latest_block: Arc::new(RwLock::new(None)),
            latest_batch: Arc::new(RwLock::new(None)),
            batch_tx,
            receipts,
        };

        (seq, batch_rx)
    }

    pub async fn run(&self) {
        info!(
            block_interval_secs = self.config.block_interval_secs,
            max_txs_per_block = self.config.max_txs_per_block,
            blocks_per_batch = self.config.blocks_per_batch,
            "sequencer started"
        );

        let mut ticker = interval(Duration::from_secs(self.config.block_interval_secs));

        loop {
            ticker.tick().await;
            if let Err(e) = self.produce_block().await {
                error!(error = %e, "block production failed");
            }
        }
    }

    async fn produce_block(&self) -> anyhow::Result<()> {
        let txs = self.mempool.drain(self.config.max_txs_per_block).await;
        if txs.is_empty() {
            return Ok(()); // nothing to do this tick
        }

        let block_number = self.next_block_number().await?;
        let parent_hash = self.parent_hash().await;
        let state_root = self.state.get_state_root_async().await;
        let tx_root = compute_tx_root(&txs);

        let header = BlockHeader {
            number: block_number,
            parent_hash,
            state_root: state_root.clone(),
            tx_root,
            timestamp: chrono::Utc::now(),
            sequencer: self.config.sequencer_id.clone(),
        };

        // Execute every transaction, collecting receipts.
        let block_hash = header.hash();
        let mut executed_txs = Vec::with_capacity(txs.len());
        let mut new_receipts = Vec::with_capacity(txs.len());

        for (i, mut tx) in txs.into_iter().enumerate() {
            let receipt = self
                .runtime
                .execute(&tx, block_number, &block_hash, i)
                .await;

            tx.status = if receipt.success {
                crate::types::TxStatus::Executed
            } else {
                crate::types::TxStatus::Failed(receipt.error.clone().unwrap_or_default())
            };

            // Persist receipt.
            self.store
                .put(storage::receipt_key(&tx.id), &receipt)?;

            new_receipts.push(receipt);
            executed_txs.push(tx);
        }

        let block = L2Block::new(header, executed_txs);

        info!(
            block = block_number,
            txs = block.tx_count(),
            hash = %block.hash,
            "block sealed"
        );

        // Persist block.
        self.store.put(storage::block_key(block_number), &block)?;
        self.store
            .put(storage::BLOCK_HEIGHT_KEY, &block_number)?;

        // Update hot state.
        *self.latest_block.write().await = Some(block.clone());
        self.receipts.write().await.extend(new_receipts);
        self.pending_blocks.write().await.push(block);

        // Maybe flush a batch.
        if self.pending_blocks.read().await.len() >= self.config.blocks_per_batch {
            self.flush_batch().await?;
        }

        Ok(())
    }

    async fn flush_batch(&self) -> anyhow::Result<()> {
        let blocks: Vec<L2Block> = self.pending_blocks.write().await.drain(..).collect();
        let batch_number = self.next_batch_number().await?;

        // Serialise for size accounting.
        let raw = bincode::serialize(&blocks)?;
        let compressed_bytes = raw.len(); // compression is in the batch submitter

        let batch = L2Batch::new(batch_number, blocks, compressed_bytes);

        info!(
            batch = batch_number,
            blocks = batch.meta.last_block - batch.meta.first_block + 1,
            txs = batch.meta.total_txs,
            "batch sealed — sending to L1 submitter"
        );

        self.store.put(storage::batch_key(batch_number), &batch)?;
        self.store.put(storage::BATCH_NUMBER_KEY, &batch_number)?;
        *self.latest_batch.write().await = Some(batch.clone());

        self.batch_tx
            .send(batch)
            .await
            .map_err(|_| anyhow::anyhow!("batch channel closed"))?;

        Ok(())
    }

    pub async fn latest_block(&self) -> Option<L2Block> {
        self.latest_block.read().await.clone()
    }

    pub async fn latest_batch(&self) -> Option<L2Batch> {
        self.latest_batch.read().await.clone()
    }

    pub async fn recent_blocks(&self, limit: usize) -> Vec<L2Block> {
        let height = match self.store.get::<u64>(storage::BLOCK_HEIGHT_KEY) {
            Ok(Some(h)) => h,
            _ => return vec![],
        };

        let start = height.saturating_sub(limit as u64 - 1);
        (start..=height)
            .filter_map(|n| self.store.get::<L2Block>(storage::block_key(n)).ok().flatten())
            .collect()
    }

    pub async fn recent_batches(&self, limit: usize) -> Vec<L2Batch> {
        let number = match self.store.get::<u64>(storage::BATCH_NUMBER_KEY) {
            Ok(Some(n)) => n,
            _ => return vec![],
        };

        let start = number.saturating_sub(limit as u64 - 1);
        (start..=number)
            .filter_map(|n| self.store.get::<L2Batch>(storage::batch_key(n)).ok().flatten())
            .collect()
    }

    async fn next_block_number(&self) -> anyhow::Result<u64> {
        let current = self
            .store
            .get::<u64>(storage::BLOCK_HEIGHT_KEY)?
            .unwrap_or(0);
        Ok(current + 1)
    }

    async fn next_batch_number(&self) -> anyhow::Result<u64> {
        let current = self
            .store
            .get::<u64>(storage::BATCH_NUMBER_KEY)?
            .unwrap_or(0);
        Ok(current + 1)
    }

    async fn parent_hash(&self) -> String {
        self.latest_block
            .read()
            .await
            .as_ref()
            .map(|b| b.hash.clone())
            .unwrap_or_else(|| "0".repeat(64))
    }
}

fn compute_tx_root(txs: &[L2Transaction]) -> String {
    use sha2::{Digest, Sha256};
    if txs.is_empty() {
        return "0".repeat(64);
    }
    let mut h = Sha256::new();
    for tx in txs {
        h.update(tx.id.as_bytes());
    }
    hex::encode(h.finalize())
}
