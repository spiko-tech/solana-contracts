use pinocchio::address::Address;

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

pub const DISCRIMINATOR_PERMISSION_CONFIG: u8 = 1;
pub const DISCRIMINATOR_USER_PERMISSION: u8 = 2;

pub const ZERO_ADDRESS: Address = Address::new_from_array([0u8; 32]);

/// Global configuration for the PermissionManager program.
///
/// Seeds: ["permission_config"]
///
/// Layout (total: 66 bytes):
///   [0]       discriminator (u8)
///   [1]       bump (u8)
///   [2..34]   admin authority (Address / 32 bytes)
///   [34..66]  pending admin (Address / 32 bytes, zero if none)
#[repr(C)]
pub struct PermissionConfig {
    pub discriminator: u8,
    pub bump: u8,
    pub admin: Address,
    pub pending_admin: Address,
}

impl PermissionConfig {
    pub const LEN: usize = core::mem::size_of::<Self>();

    pub fn from_bytes(data: &[u8]) -> Result<&Self, pinocchio::error::ProgramError> {
        if data.len() < Self::LEN {
            return Err(pinocchio::error::ProgramError::InvalidAccountData);
        }
        if data[0] != DISCRIMINATOR_PERMISSION_CONFIG {
            return Err(pinocchio::error::ProgramError::InvalidAccountData);
        }
        Ok(unsafe { &*(data.as_ptr() as *const Self) })
    }

    pub fn from_bytes_mut(data: &mut [u8]) -> Result<&mut Self, pinocchio::error::ProgramError> {
        if data.len() < Self::LEN {
            return Err(pinocchio::error::ProgramError::InvalidAccountData);
        }
        Ok(unsafe { &mut *(data.as_mut_ptr() as *mut Self) })
    }

    #[inline]
    pub fn has_pending_admin(&self) -> bool {
        self.pending_admin != ZERO_ADDRESS
    }
}

/// Stores the role bitmask for a single user address.
///
/// Seeds: ["user_perm", user_pubkey]
///
/// Layout (total: 34 bytes):
///   [0]       discriminator (u8)
///   [1]       bump (u8)
///   [2..34]   roles bitmask (32 bytes = 256 bits)
#[repr(C)]
pub struct UserPermissions {
    pub discriminator: u8,
    pub bump: u8,
    pub roles: [u8; ROLE_BITMASK_LEN],
}

impl UserPermissions {
    pub const LEN: usize = core::mem::size_of::<Self>();

    pub fn from_bytes(data: &[u8]) -> Result<&Self, pinocchio::error::ProgramError> {
        if data.len() < Self::LEN {
            return Err(pinocchio::error::ProgramError::InvalidAccountData);
        }
        if data[0] != DISCRIMINATOR_USER_PERMISSION {
            return Err(pinocchio::error::ProgramError::InvalidAccountData);
        }
        Ok(unsafe { &*(data.as_ptr() as *const Self) })
    }

    pub fn from_bytes_mut(data: &mut [u8]) -> Result<&mut Self, pinocchio::error::ProgramError> {
        if data.len() < Self::LEN {
            return Err(pinocchio::error::ProgramError::InvalidAccountData);
        }
        Ok(unsafe { &mut *(data.as_mut_ptr() as *mut Self) })
    }

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
