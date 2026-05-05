use anchor_lang::prelude::*;

use crate::constants::{ROLE_WHITELISTED, ROLE_WHITELISTED_EXT, VALID_ROLES_BITMASK};

#[account]
#[derive(InitSpace)]
pub struct PermissionConfig {
    pub admin: Pubkey,
    pub pending_admin: Pubkey,
}

#[account]
#[derive(InitSpace)]
pub struct UserPermissions {
    pub roles: u16,
}

impl UserPermissions {
    pub fn is_valid_role_mask(role: u16) -> bool {
        role != 0 && role & !VALID_ROLES_BITMASK == 0
    }

    pub fn has_role(&self, role: u16) -> bool {
        self.roles & role != 0
    }

    pub fn can_add_whitelist_role(&self, role: u16) -> bool {
        match role {
            ROLE_WHITELISTED => self.roles & ROLE_WHITELISTED_EXT == 0,
            ROLE_WHITELISTED_EXT => self.roles & ROLE_WHITELISTED == 0,
            _ => false,
        }
    }
}

pub fn has_role(permissions: &UserPermissions, role: u16) -> bool {
    permissions.has_role(role)
}

pub fn is_admin(config: &PermissionConfig, signer: &Pubkey) -> bool {
    config.admin == *signer
}
