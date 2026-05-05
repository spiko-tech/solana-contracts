pub use permission_manager::constants::*;

use anchor_lang::prelude::Pubkey;

pub fn permission_manager_program_id() -> Pubkey {
    permission_manager::ID
}

pub const MINT_AUTHORITY_SEED: &[u8] = b"mint_authority";
pub const TOKEN_CONFIG_SEED: &[u8] = b"token_config";
