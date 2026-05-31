set -euo pipefail

RPC="http://localhost:8899"
BINARY="./target/release/clutch"
DB="./demo_db"
LOG="./clutch_demo.log"

# ── Colours ──────────────────────────────────────────────────────────────────
CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BOLD='\033[1m'
RESET='\033[0m'

banner() {
  echo ""
  echo -e "${CYAN}${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
  echo -e "${CYAN}${BOLD}  $1${RESET}"
  echo -e "${CYAN}${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
}

step() {
  echo ""
  echo -e "${YELLOW}▶ $1${RESET}"
}

ok() {
  echo -e "${GREEN}✓ $1${RESET}"
}

rpc() {
  local method="$1"
  shift
  local params="${1:-[]}"
  curl -s -X POST "$RPC" \
    -H 'Content-Type: application/json' \
    -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"${method}\",\"params\":${params}}"
}

# ── Build check ──────────────────────────────────────────────────────────────
banner "Clutch Demo"

if [[ ! -f "$BINARY" ]]; then
  step "Binary not found — building (this takes ~60s the first time)"
  cargo build --release 2>&1
  ok "Build complete"
fi

# ── Cleanup previous run ─────────────────────────────────────────────────────
step "Cleaning up previous demo state"
rm -rf "$DB"
ok "State cleared"

# ── Start the node ───────────────────────────────────────────────────────────
step "Starting Clutch node in the background"
"$BINARY" \
  --port 8899 \
  --db-path "$DB" \
  --block-interval-secs 3 \
  --blocks-per-batch 3 \
  > "$LOG" 2>&1 &

NODE_PID=$!
echo "  Node PID: $NODE_PID  |  Logs: $LOG"

# Give it time to bind
sleep 2

# Verify the node is up
if ! kill -0 "$NODE_PID" 2>/dev/null; then
  echo -e "${RED}Node failed to start. Check $LOG${RESET}"
  exit 1
fi
ok "Node running"

# Ensure cleanup on exit
trap "echo ''; echo 'Stopping node...'; kill $NODE_PID 2>/dev/null; rm -rf $DB" EXIT

# ── Step 1: Chain status ──────────────────────────────────────────────────────
banner "Step 1 — Chain Status"
step "Querying chain status"
rpc "clutch_getChainStatus" | jq .
sleep 1

# ── Step 2: Faucet balance ────────────────────────────────────────────────────
banner "Step 2 — Faucet Account"
step "Checking faucet balance"
# The faucet is the system program address — pre-funded at genesis
FAUCET="11111111111111111111111111111111"
rpc "getBalance" "[\"${FAUCET}\"]" | jq .
sleep 1

# ── Step 3: Submit a transfer ─────────────────────────────────────────────────
banner "Step 3 — Submit Transaction"
step "Sending a transfer (requires a signed Solana transaction)"
cat << 'EOF'

  In a real integration you would create a Solana Transaction using
  @solana/web3.js or the Rust SDK and submit it as base58:

    curl -X POST http://localhost:8899 \
      -H 'Content-Type: application/json' \
      -d '{
        "jsonrpc": "2.0",
        "id": 1,
        "method": "sendTransaction",
        "params": ["<base58-encoded-signed-tx>"]
      }'

  The scripts/send_transfer.ts helper (TypeScript / web3.js) does this
  end-to-end. Run it with:

    npx ts-node scripts/send_transfer.ts

EOF

# ── Step 4: Wait for a block ──────────────────────────────────────────────────
banner "Step 4 — Wait for Block Production"
step "Waiting for sequencer to produce a block (~3 seconds)..."
sleep 5

# ── Step 5: Latest block ──────────────────────────────────────────────────────
banner "Step 5 — Latest Block"
rpc "clutch_getLatestBlock" | jq '{
  number:   .header.number,
  hash:     .hash,
  txs:      (.transactions | length),
  stateRoot:.header.state_root,
  timestamp:.header.timestamp
}' 2>/dev/null || echo "  (no blocks yet — mempool was empty)"

# ── Step 6: Recent blocks ─────────────────────────────────────────────────────
banner "Step 6 — Recent Blocks"
rpc "clutch_getRecentBlocks" "[10]" | jq '[.[] | {number:.header.number, hash:.hash, txs:(.transactions|length)}]'

# ── Step 7: Pending txs ───────────────────────────────────────────────────────
banner "Step 7 — Mempool"
rpc "clutch_getPendingTxs" | jq .

# ── Step 8: Latest batch ──────────────────────────────────────────────────────
banner "Step 8 — Latest Batch"
step "Waiting for batch formation (~9 seconds total from start)..."
sleep 7
rpc "clutch_getLatestBatch" | jq '{
  batchNumber: .meta.batch_number,
  blocks:      (.blocks | length),
  totalTxs:    .meta.total_txs,
  sealedAt:    .meta.sealed_at,
  status:      .status
}' 2>/dev/null || echo "  (no batch yet)"

# ── Done ──────────────────────────────────────────────────────────────────────
banner "Demo Complete"
echo ""
echo -e "  Node logs:  ${BOLD}$LOG${RESET}"
echo -e "  RPC:        ${BOLD}$RPC${RESET}"
echo ""
echo "  Useful follow-up commands:"
echo "    # Latest block (pretty)"
echo "    curl -s -X POST $RPC -H 'Content-Type:application/json' \\"
echo "      -d '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"clutch_getLatestBlock\",\"params\":[]}' | jq ."
echo ""
echo "    # Chain status"
echo "    curl -s -X POST $RPC -H 'Content-Type:application/json' \\"
echo "      -d '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"clutch_getChainStatus\",\"params\":[]}' | jq ."
echo ""
echo "Press Ctrl-C to stop the node."
wait "$NODE_PID"
