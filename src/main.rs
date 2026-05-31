use std::sync::Arc;

use clap::Parser;
use jsonrpsee::server::ServerBuilder;
use tokio::sync::RwLock;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

mod batch;
mod genesis;
mod mempool;
mod metrics;
mod rpc;
mod runtime;
mod sequencer;
mod state;
mod storage;
mod types;

use batch::BatchSubmitter;
use mempool::Mempool;
use metrics::Metrics;
use rpc::server::{RollupRpcImpl, RollupRpcServer};
use runtime::Runtime;
use sequencer::{Sequencer, SequencerConfig};
use state::StateManager;
use storage::Store;
use types::TxReceipt;

#[derive(Parser, Debug)]
#[command(
    name = "clutch",
    version,
    about = "Clutch — an educational optimistic rollup node on Solana"
)]
struct Args {
    #[arg(short, long, default_value = "8899")]
    port: u16,

    #[arg(short, long, default_value = "./clutch_db")]
    db_path: String,

    #[arg(long, default_value = "https://api.devnet.solana.com")]
    solana_rpc: String,

    #[arg(long, default_value = "2")]
    block_interval_secs: u64,

    #[arg(long, default_value = "100")]
    max_txs_per_block: usize,

    #[arg(long, default_value = "5")]
    blocks_per_batch: usize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(true)
        .with_thread_ids(false)
        .compact()
        .init();

    let args = Args::parse();

    info!(
        port = args.port,
        db = %args.db_path,
        solana_rpc = %args.solana_rpc,
        "starting Clutch rollup node"
    );

    let store = Store::open(&args.db_path)?;
    info!("storage opened at {}", args.db_path);

    let state = Arc::new(StateManager::new(store.clone()).await?);
    info!("state manager initialised (state root = {})", state.get_state_root().await);

    genesis::maybe_seed_genesis(&state).await?;

    let metrics = Metrics::new();

    let mempool = Arc::new(Mempool::new());

    let runtime = Arc::new(Runtime::new(state.clone(), metrics.clone()));

    let receipts: Arc<RwLock<Vec<TxReceipt>>> = Arc::new(RwLock::new(Vec::new()));

    let seq_config = SequencerConfig {
        block_interval_secs: args.block_interval_secs,
        max_txs_per_block: args.max_txs_per_block,
        blocks_per_batch: args.blocks_per_batch,
        sequencer_id: "clutch-sequencer-0".to_string(),
    };

    let (sequencer, batch_rx) = Sequencer::new(
        seq_config,
        mempool.clone(),
        runtime.clone(),
        state.clone(),
        store.clone(),
        receipts.clone(),
        metrics.clone(),
    );
    let sequencer = Arc::new(sequencer);

    let seq_handle = sequencer.clone();
    tokio::spawn(async move {
        seq_handle.run().await;
    });
    info!("sequencer started");

    let submitter = BatchSubmitter::new(args.solana_rpc.clone(), metrics.clone());
    tokio::spawn(async move {
        submitter.run(batch_rx).await;
    });
    info!("batch submitter started (L1 = {})", args.solana_rpc);

    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_origin(Any)
        .allow_headers(Any);

    let middleware = ServiceBuilder::new().layer(cors);

    let server = ServerBuilder::default()
        .set_http_middleware(middleware)
        .build(format!("0.0.0.0:{}", args.port))
        .await?;

    let rpc_impl = RollupRpcImpl::new(
        state.clone(),
        mempool.clone(),
        sequencer.clone(),
        store.clone(),
        receipts.clone(),
        metrics.clone(),
    );

    let handle = server.start(rpc_impl.into_rpc());

    info!("");
    info!("╔══════════════════════════════════════════════╗");
    info!("║         Clutch Rollup Node  — Ready          ║");
    info!("╠══════════════════════════════════════════════╣");
    info!("║  RPC    http://localhost:{:<19}║", args.port);
    info!("║  DB     {:<37}║", args.db_path);
    info!("╚══════════════════════════════════════════════╝");
    info!("");

    tokio::signal::ctrl_c().await?;
    warn!("shutdown signal received — stopping node");
    handle.stop()?;

    Ok(())
}
