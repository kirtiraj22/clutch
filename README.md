# Clutch

Clutch is an optimistic rollup implementation built on top of Solana.

It demonstrates the core architecture behind modern rollup systems:

* off-chain transaction execution,
* deterministic state transitions,
* block production,
* state commitments,
* batching,
* and L1 settlement.

The project is designed to make rollup internals understandable and runnable in a single Rust codebase.

---

# Overview

Traditional blockchains execute every transaction directly on the base layer.

Rollups take a different approach:

* transactions execute on an L2,
* the L2 maintains its own state and block production,
* and batches of compressed transaction data are periodically anchored to the L1.

This significantly reduces the amount of work performed on the base chain while preserving verifiability.

Clutch follows the optimistic rollup model:

* execution happens off-chain,
* state roots are computed deterministically,
* and batches can be replayed and verified independently.

---

# Architecture

```text
Client / Wallet
       │
       ▼
JSON-RPC Server
       │
       ▼
Mempool
(deduplication + staging)
       │
       ▼
Sequencer
(block production)
       │
       ▼
Runtime
(transaction execution)
       │
       ▼
State Manager
(account state + state root)
       │
       ▼
Batching Layer
(batch formation)
       │
       ▼
Solana L1
(settlement + data availability)
```

---

# Core Components

## JSON-RPC Server

Clutch exposes a Solana-inspired JSON-RPC interface for:

* transaction submission,
* account queries,
* block inspection,
* and node metrics.

---

## Mempool

The mempool temporarily stores incoming transactions before sequencing.

Responsibilities:

* transaction deduplication,
* capacity management,
* pending transaction tracking.

---

## Sequencer

The sequencer periodically:

* drains the mempool,
* executes transactions,
* produces L2 blocks,
* and groups blocks into batches.

Each block contains:

* block number,
* parent hash,
* transaction data,
* timestamp,
* and resulting state root.

---

## Runtime

The runtime is responsible for deterministic transaction execution.

Supported instruction types:

* Transfer
* Mint
* Burn
* CustomInstruction

Execution flow:

1. Validate transaction
2. Execute state transition
3. Update state root
4. Generate receipt

---

## State Manager

The state manager maintains the canonical L2 world state.

Every account mutation contributes to a deterministic global state root.

The state root acts as a cryptographic commitment to the entire chain state.

---

## Batch Submitter

The batching layer aggregates multiple blocks together and submits batch data to Solana.

This models how optimistic rollups amortise settlement costs across many transactions.

---

# Project Structure

```text
src/
├── batch/          L1 batch submission
├── genesis/        Initial chain state
├── mempool/        Transaction staging
├── metrics/        Runtime metrics
├── rpc/            JSON-RPC server
├── runtime/        Execution engine
├── sequencer/      Block production
├── state/          State manager
├── storage/        Persistence layer
├── types/          Core domain types
└── main.rs         Node bootstrap
```

---

# Running the Node

## Prerequisites

* Rust stable
* Cargo

Install Rust:

```bash
curl https://sh.rustup.rs -sSf | sh
```

---

## Build

```bash
cargo build --release
```

---

## Run

```bash
RUST_LOG=info cargo run --release
```

Default RPC endpoint:

```text
http://localhost:8899
```

---

# Example RPC Calls

## Chain Status

```bash
curl -X POST http://localhost:8899 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc":"2.0",
    "id":1,
    "method":"clutch_getChainStatus",
    "params":[]
  }'
```

---

## Latest Block

```bash
curl -X POST http://localhost:8899 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc":"2.0",
    "id":1,
    "method":"clutch_getLatestBlock",
    "params":[]
  }'
```

---

## Metrics

```bash
curl -X POST http://localhost:8899 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc":"2.0",
    "id":1,
    "method":"clutch_getMetrics",
    "params":[]
  }'
```

---

# Current Scope

Clutch is intentionally simplified for educational purposes.

The current implementation focuses on:

* execution flow,
* sequencing,
* state commitments,
* batching,
* and settlement architecture.

It does not yet implement:

* fraud proofs,
* decentralised sequencing,
* validator consensus,
* or bridge contracts.

---

# Why This Project Exists

Most rollup discussions stay theoretical.

Clutch was built to make rollup internals inspectable and understandable through a runnable system that mirrors real optimistic rollup architecture at a smaller scale.
