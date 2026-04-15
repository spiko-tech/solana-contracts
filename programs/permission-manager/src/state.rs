use pinocchio::{account::AccountView, address::Address, error::ProgramError};

use spiko_common::{
    assert_no_padding, AccountDeserialize, AccountSize, Discriminator, PdaAccount, PdaSeeds,
    Versioned,
};

pub const ROLE_MINTER: u8 = 0;
pub const ROLE_PAUSER: u8 = 1;
pub const ROLE_BURNER: u8 = 2;
pub const ROLE_WHITELISTER: u8 = 3;
pub const ROLE_WHITELISTED: u8 = 4;
pub const ROLE_REDEMPTION_EXECUTOR: u8 = 5;
pub const ROLE_MINT_APPROVER: u8 = 6;
pub const ROLE_MINT_INITIATOR: u8 = 7;
pub const MAX_ROLE_BIT: u8 = 255;

#[inline]
pub fn can_manage_role(grantor_role: u8, target_role: u8) -> bool {
    matches!(
        (grantor_role, target_role),
        (ROLE_WHITELISTER, ROLE_WHITELISTED)
    )
}

pub const ROLE_BITMASK_LEN: usize = 32;

#[inline]
pub fn has_role(bitmask: &[u8; ROLE_BITMASK_LEN], role: u8) -> bool {
    let byte_index = (role / 8) as usize;
    let bit_index = role % 8;
    bitmask[byte_index] & (1 << bit_index) != 0
}

pub const PERMISSION_CONFIG_SEED: &[u8] = b"permission_config";
pub const USER_PERMISSION_SEED: &[u8] = b"user_perm";

/// Backward-compatible discriminator constants for cross-program imports.
pub const DISCRIMINATOR_PERMISSION_CONFIG: u8 = 1;
pub const DISCRIMINATOR_USER_PERMISSION: u8 = 2;

pub const ZERO_ADDRESS: Address = Address::new_from_array([0u8; 32]);

/// Global configuration for the PermissionManager program.
///
/// Seeds: ["permission_config"]
///
/// On-chain layout (total: 67 bytes):
///   [0]       discriminator (u8) — external, trait-provided
///   [1]       version (u8) — external, trait-provided
///   [2]       bump (u8)
///   [3..35]   admin authority (Address / 32 bytes)
///   [35..67]  pending admin (Address / 32 bytes, zero if none)
#[repr(C)]
pub struct PermissionConfig {
    pub bump: u8,
    pub admin: Address,
    pub pending_admin: Address,
}

assert_no_padding!(PermissionConfig, 1 + 32 + 32);

impl Discriminator for PermissionConfig {
    const DISCRIMINATOR: u8 = DISCRIMINATOR_PERMISSION_CONFIG;
}

impl Versioned for PermissionConfig {
    const VERSION: u8 = 1;
}

impl AccountSize for PermissionConfig {
    const DATA_LEN: usize = 1 + 32 + 32; // bump + admin + pending_admin
}

impl AccountDeserialize for PermissionConfig {}

impl PdaSeeds for PermissionConfig {
    const PREFIX: &'static [u8] = PERMISSION_CONFIG_SEED;

    fn validate_pda_address(
        &self,
        account: &AccountView,
        program_id: &Address,
    ) -> Result<u8, ProgramError> {
        let (derived, bump) = Address::find_program_address(&[Self::PREFIX], program_id);
        if account.address() != &derived {
            return Err(ProgramError::InvalidSeeds);
        }
        Ok(bump)
    }
}

impl PdaAccount for PermissionConfig {
    fn bump(&self) -> u8 {
        self.bump
    }

    fn validate_self(
        &self,
        account: &AccountView,
        program_id: &Address,
    ) -> Result<(), ProgramError> {
        let (derived, _) = Address::find_program_address(&[Self::PREFIX], program_id);
        if account.address() != &derived {
            return Err(ProgramError::InvalidSeeds);
        }
        Ok(())
    }
}

impl PermissionConfig {
    #[inline]
    pub fn has_pending_admin(&self) -> bool {
        self.pending_admin != ZERO_ADDRESS
    }
}

/// Stores the role bitmask for a single user address.
///
/// Seeds: ["user_perm", user_pubkey]
///
/// On-chain layout (total: 35 bytes):
///   [0]       discriminator (u8) — external, trait-provided
///   [1]       version (u8) — external, trait-provided
///   [2]       bump (u8)
///   [3..35]   roles bitmask (32 bytes = 256 bits)
#[repr(C)]
pub struct UserPermissions {
    pub bump: u8,
    pub roles: [u8; ROLE_BITMASK_LEN],
}

assert_no_padding!(UserPermissions, 1 + 32);

impl Discriminator for UserPermissions {
    const DISCRIMINATOR: u8 = DISCRIMINATOR_USER_PERMISSION;
}

impl Versioned for UserPermissions {
    const VERSION: u8 = 1;
}

impl AccountSize for UserPermissions {
    const DATA_LEN: usize = 1 + 32; // bump + roles
}

impl AccountDeserialize for UserPermissions {}

impl UserPermissions {
    #[inline]
    pub fn has_role(&self, role: u8) -> bool {
        has_role(&self.roles, role)
    }

    #[inline]
    pub fn set_role(&mut self, role: u8) {
        let byte_index = (role / 8) as usize;
        let bit_index = role % 8;
        self.roles[byte_index] |= 1 << bit_index;
    }

    #[inline]
    pub fn clear_role(&mut self, role: u8) {
        let byte_index = (role / 8) as usize;
        let bit_index = role % 8;
        self.roles[byte_index] &= !(1 << bit_index);
    }
}
