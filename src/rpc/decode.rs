use solana_sdk::{pubkey::Pubkey, transaction::Transaction};

use crate::types::{ClutchError, L2Transaction, TransactionKind};

const SYSTEM_TRANSFER_DISCRIMINANT: u32 = 2;

pub fn decode_transaction(tx: &Transaction, raw: Vec<u8>) -> Result<L2Transaction, ClutchError> {
    tx.verify()
        .map_err(|_| ClutchError::InvalidSignature)?;

    let sig = tx
        .signatures
        .first()
        .ok_or(ClutchError::InvalidSignature)?
        .to_string();

    let from = *tx
        .message
        .account_keys
        .first()
        .ok_or(ClutchError::InvalidSignature)?;

    let nonce = extract_nonce(tx);

    let kind = decode_kind(tx, &from)?;

    Ok(L2Transaction::new(sig, from, kind, nonce, raw))
}

fn decode_kind(tx: &Transaction, from: &Pubkey) -> Result<TransactionKind, ClutchError> {
    let ix = match tx.message.instructions.first() {
        Some(ix) => ix,
        None => return Ok(TransactionKind::CustomInstruction {
            program_id: *from,
            data: vec![],
        }),
    };

    let data = &ix.data;

    if data.len() >= 4 {
        let disc = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);

        if disc == SYSTEM_TRANSFER_DISCRIMINANT && data.len() >= 12 {
            let lamports = u64::from_le_bytes(data[4..12].try_into().unwrap());
            let to = tx
                .message
                .account_keys
                .get(*ix.accounts.get(1).unwrap_or(&0) as usize)
                .copied()
                .unwrap_or(*from);

            return Ok(TransactionKind::Transfer { to, lamports });
        }

        match disc {
            0x10000000 if data.len() >= 12 => {
                let lamports = u64::from_le_bytes(data[4..12].try_into().unwrap());
                let to = tx
                    .message
                    .account_keys
                    .get(*ix.accounts.first().unwrap_or(&0) as usize)
                    .copied()
                    .unwrap_or(*from);
                return Ok(TransactionKind::Mint { to, lamports });
            }
            0x20000000 if data.len() >= 12 => {
                let lamports = u64::from_le_bytes(data[4..12].try_into().unwrap());
                return Ok(TransactionKind::Burn { lamports });
            }
            _ => {}
        }
    }

    let program_key_idx = ix.program_id_index as usize;
    let program_id = tx
        .message
        .account_keys
        .get(program_key_idx)
        .copied()
        .unwrap_or(*from);

    Ok(TransactionKind::CustomInstruction {
        program_id,
        data: data.to_vec(),
    })
}

fn extract_nonce(tx: &Transaction) -> u64 {
    tx.message
        .instructions
        .first()
        .and_then(|ix| {
            if ix.data.len() >= 20 {
                Some(u64::from_le_bytes(ix.data[12..20].try_into().ok()?))
            } else {
                None
            }
        })
        .unwrap_or(0)
}
