# Clutch

**An educational optimistic rollup execution engine on Solana.**

Clutch is a complete, runnable L2 rollup prototype built in Rust with a production-grade Next.js frontend. It demonstrates the core concepts of an optimistic rollup — off-chain execution, Merkle state commitments, batch sequencing, and L1 settlement — without the production complexity of fraud proofs or decentralised consensus.

---

## Architecture

```
Client / Wallet
      │  JSON-RPC (Solana-compatible)
      ▼
   Mempool          dedup · cap · FIFO
      │  drain every N seconds
      ▼
  Sequencer         block production · timestamps · parent hashes
      │  execute
      ▼
  Runtime           Transfer · Mint · Burn · CustomInstruction
      │  write-through
      ▼
 StateManager        write-through cache → RocksDB
                     SHA-256 Merkle state root
      │  batch every M blocks
      ▼
BatchSubmitter       bincode encode → Solana instruction → L1
      │
      ▼
  Solana (L1)        data availability · challenge window
```

---

## Project Structure

```
clutch/                     Rust backend
  src/
    main.rs                 CLI + wiring
    genesis.rs              First-boot state seeding
    metrics.rs              Atomic counters
    types/                  All domain types
      error.rs              ClutchError (thiserror)
      transaction.rs        TransactionKind enum
      block.rs              L2Block + BlockHeader
      batch.rs              L2Batch + BatchMeta
      account.rs            L2Account
      receipt.rs            TxReceipt
    storage/                RocksDB abstraction
    state/                  Merkle-rooted StateManager
    mempool/                Pending tx queue
    runtime/                Execution engine
    sequencer/              Block production + batching
    batch/                  L1 submission
    rpc/                    JSON-RPC server + decoder
  config/
    default.toml            Default node config
  scripts/
    demo.sh                 Full interactive demo
    send_transfer.ts        TypeScript tx submission

```

---

## Quick Start

### Backend

```bash
# Prerequisites: Rust stable (https://rustup.rs)
cd clutch

# Build
cargo build --release

# Run (defaults: port 8899, DB ./clutch_db, devnet L1)
cargo run --release

# Or with custom config
cargo run --release -- \
  --port 8899 \
  --db-path ./clutch_db \
  --solana-rpc https://api.devnet.solana.com \
  --block-interval-secs 2 \
  --blocks-per-batch 5
```

Set `RUST_LOG=debug` for verbose output, `RUST_LOG=clutch::sequencer=trace` for sequencer traces.

## RPC Reference

### Solana-compatible

| Method | Description |
|---|---|
| `getBalance` | L2 lamport balance |
| `getAccountInfo` | Full account data |
| `sendTransaction` | Submit a signed transaction |
| `getLatestBlockhash` | For transaction construction |
| `simulateTransaction` | Dry-run |
| `getTransaction` | Receipt by signature |

### Clutch-native

| Method | Description |
|---|---|
| `clutch_getChainStatus` | Height, batch count, state root, pending count |
| `clutch_getLatestBlock` | Most recent L2 block |
| `clutch_getLatestBatch` | Most recent L1 batch |
| `clutch_getRecentBlocks` | Last N blocks |
| `clutch_getRecentBatches` | Last N batches |
| `clutch_getPendingTxs` | Mempool contents |
| `clutch_getTransactionReceipt` | Execution receipt with logs |
| `clutch_getMetrics` | Node metrics snapshot |

---

## Instruction Set

| Instruction | Description |
|---|---|
| `Transfer { to, lamports }` | Move lamports between accounts |
| `Mint { to, lamports }` | Credit tokens (authority-gated) |
| `Burn { lamports }` | Destroy **tokens** from signer |
| `CustomInstruction { program_id, data }` | Extensibility hook |
