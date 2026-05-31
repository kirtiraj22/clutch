use std::sync::Arc;

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use crate::metrics::Metrics;
use crate::types::L2Batch;

pub struct BatchSubmitter {
    rpc: RpcClient,
    program_id: Pubkey,
    authority: Keypair,
    metrics: Arc<Metrics>,
}

impl BatchSubmitter {
    pub fn new(solana_rpc_url: String, metrics: Arc<Metrics>) -> Self {
        Self {
            rpc: RpcClient::new_with_commitment(solana_rpc_url, CommitmentConfig::confirmed()),
            program_id: Pubkey::new_unique(),
            authority: Keypair::new(),
            metrics,
        }
    }

    pub async fn run(&self, mut rx: mpsc::Receiver<L2Batch>) {
        info!("batch submitter listening for batches");
        while let Some(batch) = rx.recv().await {
            let batch_number = batch.meta.batch_number;
            info!(
                batch = batch_number,
                blocks = batch.blocks.len(),
                txs = batch.meta.total_txs,
                "submitting batch to L1"
            );
            match self.submit(&batch).await {
                Ok(sig) => {
                    self.metrics.inc_batches_submitted();
                    info!(batch = batch_number, l1_signature = %sig, "batch confirmed on L1");
                }
                Err(e) => {
                    self.metrics.inc_batches_failed();
                    error!(batch = batch_number, error = %e, "L1 submission failed");
                }
            }
        }
        warn!("batch channel closed — submitter exiting");
    }

    async fn submit(&self, batch: &L2Batch) -> anyhow::Result<String> {
        let payload = bincode::serialize(batch)?;
        let instruction =
            solana_sdk::instruction::Instruction::new_with_bytes(self.program_id, &payload, vec![]);
        let blockhash = self.rpc.get_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&self.authority.pubkey()),
            &[&self.authority],
            blockhash,
        );
        let sig = self.rpc.send_and_confirm_transaction(&tx).await?;
        Ok(sig.to_string())
    }
}
