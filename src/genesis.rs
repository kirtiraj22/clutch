use solana_sdk::pubkey::Pubkey;
use tracing::info;

use crate::state::StateManager;
use crate::types::L2Account;
pub const FAUCET_LAMPORTS: u64 = 1_000_000 * 1_000_000_000;

pub const FAUCET_PUBKEY: &str = "11111111111111111111111111111111";

pub async fn maybe_seed_genesis(state: &StateManager) -> anyhow::Result<()> {
    let faucet: Pubkey = FAUCET_PUBKEY.parse().expect("valid system pubkey");

    if state.get_account(&faucet).await.is_some() {
        return Ok(());
    }

    info!("seeding genesis state");

    state
        .update_account(&faucet, L2Account::with_lamports(FAUCET_LAMPORTS))
        .await?;

    info!(
        faucet = %faucet,
        lamports = FAUCET_LAMPORTS,
        "genesis complete — faucet pre-funded"
    );

    Ok(())
}
