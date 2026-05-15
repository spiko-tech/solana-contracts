use anchor_lang::prelude::*;

use crate::constants::*;
use crate::errors::TransferHookError;

#[account]
#[derive(InitSpace)]
pub struct HookConfig {
    pub whitelist_authority: Pubkey,
    pub admin: Pubkey,
    pub bump: u8,
}

impl HookConfig {
    pub fn validate_transfer(&self, source_role: u8, dest_role: u8) -> Result<()> {
        let allowed = match source_role {
            ROLE_WHITELISTED => dest_role == ROLE_WHITELISTED || dest_role == ROLE_WHITELISTED_GATE,
            ROLE_WHITELISTED_EXT => {
                dest_role == ROLE_WHITELISTED_EXT
                    || dest_role == ROLE_WHITELISTED
                    || dest_role == ROLE_WHITELISTED_GATE
            }
            ROLE_WHITELISTED_GATE => {
                dest_role == ROLE_WHITELISTED_EXT
                    || dest_role == ROLE_WHITELISTED
                    || dest_role == ROLE_WHITELISTED_GATE
            }
            _ => false,
        };

        require!(allowed, TransferHookError::Unauthorized);
        Ok(())
    }
}

#[account]
#[derive(InitSpace)]
pub struct WhitelistState {
    pub role: u8,
    pub bump: u8,
}
