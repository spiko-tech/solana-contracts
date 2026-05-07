use anchor_lang::prelude::*;

pub use permission_manager::state::{has_role, is_admin, PermissionConfig, UserPermissions};

#[account]
#[derive(InitSpace)]
pub struct TokenConfig {
    pub paused: bool,
    pub permission_manager: Pubkey,
    pub mint: Pubkey,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct MintAuthority {
    pub bump: u8,
}
