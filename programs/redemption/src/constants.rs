pub use permission_manager::constants::*;

use anchor_lang::prelude::Pubkey;

pub fn permission_manager_program_id() -> Pubkey {
    permission_manager::ID
}

pub const REDEMPTION_CONFIG_SEED: &[u8] = b"redemption_config";
pub const REDEMPTION_VAULT_SEED: &[u8] = b"redemption_vault";
pub const REDEMPTION_OPERATION_SEED: &[u8] = b"redemption_op";
pub const VAULT_AUTHORITY_SEED: &[u8] = b"vault_authority";

pub const STATUS_PENDING: u8 = 1;
pub const STATUS_DONE: u8 = 2;
pub const STATUS_CANCELED: u8 = 3;
