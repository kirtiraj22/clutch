import {
  Connection,
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction,
  sendAndConfirmTransaction,
} from "@solana/web3.js";

const CLUTCH_RPC = "http://localhost:8899";
const TRANSFER_AMOUNT = 0.1 * LAMPORTS_PER_SOL; // 0.1 SOL in lamports

async function printBalance(conn: Connection, label: string, key: PublicKey) {
  const resp = await conn.getBalance(key);
  console.log(`  ${label}: ${resp} lamports`);
}


async function main() {
  console.log("─────────────────────────────────────────────");
  console.log("  Clutch Rollup — Transfer Demo");
  console.log("─────────────────────────────────────────────");
  console.log(`  RPC: ${CLUTCH_RPC}`);

  const conn = new Connection(CLUTCH_RPC, "confirmed");

  const sender = Keypair.generate();
  const recipient = Keypair.generate();

  console.log(`\n  Sender:    ${sender.publicKey.toBase58()}`);
  console.log(`  Recipient: ${recipient.publicKey.toBase58()}`);

  console.log("\n[Before]");
  await printBalance(conn, "Sender   ", sender.publicKey);
  await printBalance(conn, "Recipient", recipient.publicKey);

  const { blockhash } = await conn.getLatestBlockhash();

  const tx = new Transaction({ recentBlockhash: blockhash, feePayer: sender.publicKey });

  tx.add(
    SystemProgram.transfer({
      fromPubkey: sender.publicKey,
      toPubkey: recipient.publicKey,
      lamports: TRANSFER_AMOUNT,
    })
  );

 console.log(`\n[Sending] ${TRANSFER_AMOUNT} lamports`);

  try {
    const sig = await sendAndConfirmTransaction(conn, tx, [sender]);
    console.log(`\n  ✓ Signature: ${sig}`);
    console.log("\n[After]");
    await printBalance(conn, "Sender   ", sender.publicKey);
    await printBalance(conn, "Recipient", recipient.publicKey);

    console.log("\n[Receipt]");
    const receipt: any = await fetch(CLUTCH_RPC, {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({
    jsonrpc: "2.0",
    id: 1,
    method: "clutch_getTransactionReceipt",
    params: [sig],
  }),
}).then((r) => r.json());

console.log(JSON.stringify(receipt?.result ?? receipt, null, 2));
  } catch (err: any) {
    console.error(`\n  ✗ Error: ${err.message ?? err}`);
  }
}

main().catch(console.error);
