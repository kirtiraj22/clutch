use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use serde_json::{json, Value};

#[derive(Default)]
pub struct Metrics {
    pub txs_received: AtomicU64,
    pub txs_executed: AtomicU64,
    pub txs_failed: AtomicU64,
    pub blocks_produced: AtomicU64,
    pub batches_submitted: AtomicU64,
    pub batches_failed: AtomicU64,
}

impl Metrics {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn inc_txs_received(&self) {
        self.txs_received.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_txs_executed(&self) {
        self.txs_executed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_txs_failed(&self) {
        self.txs_failed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_blocks_produced(&self) {
        self.blocks_produced.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_batches_submitted(&self) {
        self.batches_submitted.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_batches_failed(&self) {
        self.batches_failed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> Value {
        let executed = self.txs_executed.load(Ordering::Relaxed);
        let failed = self.txs_failed.load(Ordering::Relaxed);
        let total = executed + failed;
        let success_rate = if total > 0 {
            format!("{:.1}%", (executed as f64 / total as f64) * 100.0)
        } else {
            "n/a".to_string()
        };

        json!({
            "transactions": {
                "received":    self.txs_received.load(Ordering::Relaxed),
                "executed":    executed,
                "failed":      failed,
                "successRate": success_rate,
            },
            "blocks": {
                "produced": self.blocks_produced.load(Ordering::Relaxed),
            },
            "batches": {
                "submitted": self.batches_submitted.load(Ordering::Relaxed),
                "failed":    self.batches_failed.load(Ordering::Relaxed),
            },
        })
    }
}
