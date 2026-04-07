use pinocchio::address::Address;

pub const MINTER_CONFIG_SEED: &[u8] = b"minter_config";
pub const DAILY_LIMIT_SEED: &[u8] = b"daily_limit";
pub const MINT_OPERATION_SEED: &[u8] = b"mint_op";

pub const DISCRIMINATOR_MINTER_CONFIG: u8 = 1;
pub const DISCRIMINATOR_DAILY_LIMIT: u8 = 2;
pub const DISCRIMINATOR_MINT_OPERATION: u8 = 3;

pub const STATUS_NULL: u8 = 0;
pub const STATUS_PENDING: u8 = 1;
pub const STATUS_DONE: u8 = 2;

pub const SECONDS_PER_DAY: i64 = 86_400;

/// Global configuration for the Minter program.
///
/// Seeds: ["minter_config"]
///
/// Layout (total: 42 bytes):
///   [0]       discriminator (u8)
///   [1]       bump (u8)
///   [2..10]   max_delay (i64, little-endian, seconds)
///   [10..42]  permission_manager program ID (Address / 32 bytes)
///
/// Note: repr(C) with [u8;8] fields avoids alignment padding.
#[repr(C)]
pub struct MinterConfig {
    pub discriminator: u8,
    pub bump: u8,
    max_delay: [u8; 8],
    pub permission_manager: Address,
}

impl MinterConfig {
    pub const LEN: usize = 42; // 1 + 1 + 8 + 32

    pub fn from_bytes(data: &[u8]) -> Result<&Self, pinocchio::error::ProgramError> {
        if data.len() < Self::LEN {
            return Err(pinocchio::error::ProgramError::InvalidAccountData);
        }
        if data[0] != DISCRIMINATOR_MINTER_CONFIG {
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

    pub fn max_delay(&self) -> i64 {
        i64::from_le_bytes(self.max_delay)
    }

    pub fn set_max_delay(&mut self, value: i64) {
        self.max_delay = value.to_le_bytes();
    }
}

/// Tracks the daily mint limit and usage for a specific token.
///
/// Seeds: ["daily_limit", mint_pubkey]
///
/// Layout (total: 26 bytes):
///   [0]       discriminator (u8)
///   [1]       bump (u8)
///   [2..10]   limit (u64, little-endian)
///   [10..18]  used_amount (u64, little-endian)
///   [18..26]  last_day (i64, little-endian) -- floor(timestamp / 86400)
#[repr(C)]
pub struct DailyLimit {
    pub discriminator: u8,
    pub bump: u8,
    limit: [u8; 8],
    used_amount: [u8; 8],
    last_day: [u8; 8],
}

impl DailyLimit {
    pub const LEN: usize = 26; // 1 + 1 + 8 + 8 + 8

    pub fn from_bytes(data: &[u8]) -> Result<&Self, pinocchio::error::ProgramError> {
        if data.len() < Self::LEN {
            return Err(pinocchio::error::ProgramError::InvalidAccountData);
        }
        if data[0] != DISCRIMINATOR_DAILY_LIMIT {
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

    pub fn limit(&self) -> u64 {
        u64::from_le_bytes(self.limit)
    }

    pub fn set_limit(&mut self, value: u64) {
        self.limit = value.to_le_bytes();
    }

    pub fn used_amount(&self) -> u64 {
        u64::from_le_bytes(self.used_amount)
    }

    pub fn set_used_amount(&mut self, value: u64) {
        self.used_amount = value.to_le_bytes();
    }

    pub fn last_day(&self) -> i64 {
        i64::from_le_bytes(self.last_day)
    }

    pub fn set_last_day(&mut self, value: i64) {
        self.last_day = value.to_le_bytes();
    }
}

/// Tracks the status and deadline of a single mint operation.
///
/// Seeds: ["mint_op", operation_id (32 bytes)]
///
/// Layout (total: 11 bytes):
///   [0]       discriminator (u8)
///   [1]       bump (u8)
///   [2]       status (u8): NULL=0, PENDING=1, DONE=2
///   [3..11]   deadline (i64, little-endian, unix timestamp)
#[repr(C, packed)]
pub struct MintOperation {
    pub discriminator: u8,
    pub bump: u8,
    pub status: u8,
    deadline: [u8; 8],
}

impl MintOperation {
    pub const LEN: usize = 11; // 1 + 1 + 1 + 8

    pub fn from_bytes(data: &[u8]) -> Result<&Self, pinocchio::error::ProgramError> {
        if data.len() < Self::LEN {
            return Err(pinocchio::error::ProgramError::InvalidAccountData);
        }
        if data[0] != DISCRIMINATOR_MINT_OPERATION {
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
