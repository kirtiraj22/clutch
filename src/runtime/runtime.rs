use std::sync::Arc;

use solana_sdk::pubkey::Pubkey;
use tracing::{info, instrument, warn};

use crate::metrics::Metrics;
use crate::state::StateManager;
use crate::types::{ClutchError, L2Account, L2Transaction, TransactionKind, TxReceipt};

pub const MINT_AUTHORITY: &str = "11111111111111111111111111111111";

#[derive(Clone)]
pub struct Runtime {
    state: Arc<StateManager>,
    metrics: Arc<Metrics>,
}

impl Runtime {
    pub fn new(state: Arc<StateManager>, metrics: Arc<Metrics>) -> Self {
        Self { state, metrics }
    }

    #[instrument(skip(self, tx), fields(tx_id = %tx.id, from = %tx.from))]
    pub async fn execute(
        &self,
        tx: &L2Transaction,
        block_number: u64,
        block_hash: &str,
        tx_index: usize,
    ) -> TxReceipt {
        match self.run(tx).await {
            Ok(logs) => {
                info!(tx_id = %tx.id, "transaction executed successfully");
                TxReceipt::success(
                    tx.id.clone(),
                    block_number,
                    block_hash.to_string(),
                    tx_index,
                    logs,
                )
            }
            Err(e) => {
                warn!(tx_id = %tx.id, error = %e, "transaction execution failed");
                TxReceipt::failure(
                    tx.id.clone(),
                    block_number,
                    block_hash.to_string(),
                    tx_index,
                    e.to_string(),
                )
            }
        }
    }

    async fn run(&self, tx: &L2Transaction) -> Result<Vec<String>, ClutchError> {
        self.validate_nonce(tx).await?;
        let logs = match &tx.kind {
            TransactionKind::Transfer { to, lamports } => {
                self.transfer(&tx.from, to, *lamports).await?
            }
            TransactionKind::Mint { to, lamports } => self.mint(&tx.from, to, *lamports).await?,
            TransactionKind::Burn { lamports } => self.burn(&tx.from, *lamports).await?,
            TransactionKind::CustomInstruction { program_id, data } => {
                self.custom_instruction(&tx.from, program_id, data).await?
            }
        };
        self.bump_nonce(&tx.from).await?;
        Ok(logs)
    }

    async fn validate_nonce(&self, tx: &L2Transaction) -> Result<(), ClutchError> {
        let expected = self.state.get_nonce(&tx.from).await;
        if tx.nonce != expected {
            return Err(ClutchError::InvalidNonce {
                expected,
                got: tx.nonce,
            });
        }
        Ok(())
    }

    async fn bump_nonce(&self, pubkey: &Pubkey) -> Result<(), ClutchError> {
        let mut acct = self
            .state
            .get_account(pubkey)
            .await
            .unwrap_or_else(L2Account::new_system_owned);
        acct.nonce += 1;
        self.state.update_account(pubkey, acct).await
    }

    async fn transfer(
        &self,
        from: &Pubkey,
        to: &Pubkey,
        lamports: u64,
    ) -> Result<Vec<String>, ClutchError> {
        let mut from_acct = self
            .state
            .get_account(from)
            .await
            .unwrap_or_else(L2Account::new_system_owned);
        if from_acct.lamports < lamports {
            return Err(ClutchError::InsufficientFunds {
                needed: lamports,
                available: from_acct.lamports,
            });
        }
        let mut to_acct = self
            .state
            .get_account(to)
            .await
            .unwrap_or_else(L2Account::new_system_owned);
        from_acct.lamports -= lamports;
        to_acct.lamports += lamports;
        self.state.update_account(from, from_acct).await?;
        self.state.update_account(to, to_acct).await?;
        Ok(vec![format!(
            "Transfer: {} lamports from {} to {}",
            lamports, from, to
        )])
    }

    async fn mint(
        &self,
        authority: &Pubkey,
        to: &Pubkey,
        lamports: u64,
    ) -> Result<Vec<String>, ClutchError> {
        if authority.to_string() != MINT_AUTHORITY {
            return Err(ClutchError::InvalidSignature);
        }
        let mut to_acct = self
            .state
            .get_account(to)
            .await
            .unwrap_or_else(L2Account::new_system_owned);
        to_acct.lamports += lamports;
        self.state.update_account(to, to_acct).await?;
        Ok(vec![format!("Mint: {} lamports to {}", lamports, to)])
    }

    async fn burn(&self, from: &Pubkey, lamports: u64) -> Result<Vec<String>, ClutchError> {
        let mut acct = self
            .state
            .get_account(from)
            .await
            .unwrap_or_else(L2Account::new_system_owned);
        if acct.lamports < lamports {
            return Err(ClutchError::InsufficientFunds {
                needed: lamports,
                available: acct.lamports,
            });
        }
        acct.lamports -= lamports;
        self.state.update_account(from, acct).await?;
        Ok(vec![format!("Burn: {} lamports from {}", lamports, from)])
    }

    async fn custom_instruction(
        &self,
        _from: &Pubkey,
        program_id: &Pubkey,
        data: &[u8],
    ) -> Result<Vec<String>, ClutchError> {
        warn!(program = %program_id, data_len = data.len(), "custom instruction (no-op)");
        Ok(vec![format!(
            "CustomInstruction: program={} data_len={}",
            program_id,
            data.len()
        )])
    }
}
