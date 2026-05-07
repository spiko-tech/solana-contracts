use anchor_lang::prelude::*;

use crate::constants::*;
use crate::errors::TransferHookError;

#[account]
#[derive(InitSpace)]
pub struct HookConfig {
    pub paused: bool,
    pub permission_manager: Pubkey,
    pub bump: u8,
}

impl HookConfig {
    pub fn seeds(mint: &Pubkey) -> [&[u8]; 2] {
        [HOOK_CONFIG_SEED, mint.as_ref()]
    }

    /// Validates the transfer permission matrix:
    /// - Token must not be paused
    /// - Source must be WHITELISTED or WHITELISTED_EXT
    /// - Destination must be WHITELISTED or WHITELISTED_EXT
    /// - If source is only WHITELISTED_EXT (not WHITELISTED), destination must be WHITELISTED
    pub fn validate_transfer(&self, source_roles: u16, dest_roles: u16) -> Result<()> {
        require!(!self.paused, TransferHookError::Paused);

        let source_whitelisted = source_roles & ROLE_WHITELISTED != 0;
        let source_ext = source_roles & ROLE_WHITELISTED_EXT != 0;
        let dest_whitelisted = dest_roles & ROLE_WHITELISTED != 0;
        let dest_ext = dest_roles & ROLE_WHITELISTED_EXT != 0;

        require!(
            source_whitelisted || source_ext,
            TransferHookError::Unauthorized
        );
        require!(
            dest_whitelisted || dest_ext,
            TransferHookError::Unauthorized
        );

        // WHITELISTED (non-ext) can only send to WHITELISTED
        if source_whitelisted && !source_ext {
            require!(dest_whitelisted, TransferHookError::Unauthorized);
        }

        Ok(())
    }
}
