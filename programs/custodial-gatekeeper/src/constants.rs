pub use permission_manager::constants::*;

use anchor_lang::prelude::Pubkey;

pub fn permission_manager_program_id() -> Pubkey {
    permission_manager::ID
}

/// The spiko-transfer-hook program ID (from deploy keypair).
pub fn transfer_hook_program_id() -> Pubkey {
    // 21Qu5pfKsxFpmDpwrXq1ZjVxCDW5kA9jrtBuMeQCNh86
    Pubkey::new_from_array([
        0x0e, 0xf6, 0xb5, 0x5d, 0xe8, 0xfc, 0x39, 0x3a, 0x3b, 0xc8, 0x54, 0x5c, 0xfb, 0xfb, 0xb1,
        0xc0, 0x99, 0xf3, 0xb9, 0xc7, 0xff, 0x9f, 0xde, 0x16, 0x06, 0xd3, 0xe4, 0xb4, 0x88, 0x47,
        0xda, 0xd3,
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
