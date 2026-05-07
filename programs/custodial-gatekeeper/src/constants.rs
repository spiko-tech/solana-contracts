pub use permission_manager::constants::*;

use anchor_lang::prelude::Pubkey;

pub fn permission_manager_program_id() -> Pubkey {
    permission_manager::ID
}

/// The spiko-transfer-hook program ID (from deploy keypair).
pub fn transfer_hook_program_id() -> Pubkey {
    // 7DXckwPHM1ktduwLXWxsn87hWrmyUVKDNNst5ycAj8VU
    Pubkey::new_from_array([
        0x5c, 0x5b, 0xb5, 0x9f, 0x78, 0x56, 0xba, 0xef, 0x8f, 0x72, 0xd7, 0x00, 0x6d, 0x85, 0x08,
        0xf8, 0xfa, 0xc9, 0xc0, 0x5c, 0xc0, 0xa3, 0x83, 0x8a, 0x55, 0xae, 0xc8, 0x11, 0xa7, 0xd0,
        0x02, 0x6f,
    ])
}

pub const HOOK_CONFIG_SEED: &[u8] = b"hook_config";
pub const EXTRA_ACCOUNT_METAS_SEED: &[u8] = b"extra-account-metas";

pub const GATEKEEPER_CONFIG_SEED: &[u8] = b"gatekeeper_config";
pub const WITHDRAWAL_DAILY_LIMIT_SEED: &[u8] = b"withdrawal_limit";
pub const WITHDRAWAL_OPERATION_SEED: &[u8] = b"withdrawal_op";
pub const VAULT_SEED: &[u8] = b"vault";

pub const SECONDS_PER_DAY: i64 = 86400;

pub const STATUS_PENDING: u8 = 1;
pub const STATUS_DONE: u8 = 2;
pub const STATUS_CANCELED: u8 = 3;
