pub use permission_manager::constants::*;

use anchor_lang::prelude::Pubkey;

pub fn permission_manager_program_id() -> Pubkey {
    permission_manager::ID
}

pub const MINTER_CONFIG_SEED: &[u8] = b"minter_config";
pub const MINT_DAILY_LIMIT_SEED: &[u8] = b"mint_daily_limit";
pub const MINT_OPERATION_SEED: &[u8] = b"mint_op";
pub const SECONDS_PER_DAY: i64 = 86400;

pub const STATUS_PENDING: u8 = 1;
pub const STATUS_DONE: u8 = 2;
pub const STATUS_CANCELED: u8 = 3;
