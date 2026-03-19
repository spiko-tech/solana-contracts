use pinocchio::address::Address;

// -----------------------------------------------------------------
// PDA Seeds
// -----------------------------------------------------------------

pub const REDEMPTION_CONFIG_SEED: &[u8] = b"redemption_config";
pub const TOKEN_MINIMUM_SEED: &[u8] = b"minimum";
pub const REDEMPTION_OPERATION_SEED: &[u8] = b"redemption_op";
pub const VAULT_SEED: &[u8] = b"vault";

// -----------------------------------------------------------------
// Account discriminators
// -----------------------------------------------------------------

pub const DISCRIMINATOR_REDEMPTION_CONFIG: u8 = 1;
pub const DISCRIMINATOR_TOKEN_MINIMUM: u8 = 2;
pub const DISCRIMINATOR_REDEMPTION_OPERATION: u8 = 4;

// -----------------------------------------------------------------
// Operation status values
// -----------------------------------------------------------------

pub const STATUS_NULL: u8 = 0;
pub const STATUS_PENDING: u8 = 1;
pub const STATUS_EXECUTED: u8 = 2;
pub const STATUS_CANCELED: u8 = 3;

// -----------------------------------------------------------------
// Constants
// -----------------------------------------------------------------

/// Maximum delay for redemption operations (14 days in seconds).
pub const MAX_DELAY: i64 = 14 * 24 * 60 * 60; // 1_209_600

// -----------------------------------------------------------------
// RedemptionConfig -- Singleton configuration
// -----------------------------------------------------------------

/// Global configuration for the Redemption program.
///
/// Seeds: ["redemption_config"]
///
/// Layout (total: 34 bytes):
///   [0]       discriminator (u8)
///   [1]       bump (u8)
///   [2..34]   permission_manager program ID (Address / 32 bytes)
#[repr(C)]
pub struct RedemptionConfig {
    pub discriminator: u8,
    pub bump: u8,
    pub permission_manager: Address,
}

impl RedemptionConfig {
    pub const LEN: usize = core::mem::size_of::<Self>();

    pub fn from_bytes(data: &[u8]) -> Result<&Self, pinocchio::error::ProgramError> {
        if data.len() < Self::LEN {
            return Err(pinocchio::error::ProgramError::InvalidAccountData);
        }
        if data[0] != DISCRIMINATOR_REDEMPTION_CONFIG {
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
}

// -----------------------------------------------------------------
// TokenMinimum -- Per-token minimum redemption amount
// -----------------------------------------------------------------

/// Stores the minimum redemption amount for a specific token.
///
/// Seeds: ["minimum", mint_pubkey]
///
/// Layout (total: 10 bytes):
///   [0]       discriminator (u8)
///   [1]       bump (u8)
///   [2..10]   minimum_amount (u64, little-endian)
#[repr(C)]
pub struct TokenMinimum {
    pub discriminator: u8,
    pub bump: u8,
    minimum_amount: [u8; 8],
}

impl TokenMinimum {
    pub const LEN: usize = core::mem::size_of::<Self>();

    pub fn from_bytes(data: &[u8]) -> Result<&Self, pinocchio::error::ProgramError> {
        if data.len() < Self::LEN {
            return Err(pinocchio::error::ProgramError::InvalidAccountData);
        }
        if data[0] != DISCRIMINATOR_TOKEN_MINIMUM {
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

    pub fn minimum_amount(&self) -> u64 {
        u64::from_le_bytes(self.minimum_amount)
    }

    pub fn set_minimum_amount(&mut self, value: u64) {
        self.minimum_amount = value.to_le_bytes();
    }
}

// -----------------------------------------------------------------
// RedemptionOperation -- Per-operation state
// -----------------------------------------------------------------

/// Tracks the status of a single redemption operation.
///
/// Seeds: ["redemption_op", operation_id (32 bytes)]
///
/// Layout (total: 44 bytes):
///   [0]       discriminator (u8)
///   [1]       bump (u8)
///   [2]       status (u8): NULL=0, PENDING=1, EXECUTED=2, CANCELED=3
///   [3]       _padding (u8)
///   [4..12]   deadline (i64, little-endian, unix timestamp)
///   [12..44]  user address (Address / 32 bytes, for refund on cancel)
#[repr(C)]
pub struct RedemptionOperation {
    pub discriminator: u8,
    pub bump: u8,
    pub status: u8,
    _padding: u8,
    deadline: [u8; 8],
    pub user: Address,
}

impl RedemptionOperation {
    pub const LEN: usize = core::mem::size_of::<Self>();

    pub fn from_bytes(data: &[u8]) -> Result<&Self, pinocchio::error::ProgramError> {
        if data.len() < Self::LEN {
            return Err(pinocchio::error::ProgramError::InvalidAccountData);
        }
        if data[0] != DISCRIMINATOR_REDEMPTION_OPERATION {
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

    pub fn deadline(&self) -> i64 {
        i64::from_le_bytes(self.deadline)
    }

    pub fn set_deadline(&mut self, value: i64) {
        self.deadline = value.to_le_bytes();
    }
}
