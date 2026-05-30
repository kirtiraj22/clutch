use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2Account {
    pub lamports: u64,
    pub data: Vec<u8>,
    pub owner: Pubkey,
    pub executable: bool,
    pub nonce: u64,
}

impl L2Account {
    pub fn new_system_owned() -> Self {
        Self {
            lamports: 0,
            data: vec![],
            owner: Pubkey::from_str_const("11111111111111111111111111111111"),
            executable: false,
            nonce: 0,
        }
    }

    pub fn with_lamports(lamports: u64) -> Self {
        Self {
            lamports,
            ..Self::new_system_owned()
        }
    }
}
