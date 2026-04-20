use codama::CodamaAccount;
use pinocchio::{account::AccountView, address::Address, error::ProgramError};

use spiko_common::{
    assert_no_padding, AccountDeserialize, AccountSize, Discriminator, PdaAccount, PdaSeeds,
    Versioned,
};

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
/// On-chain layout (total: 43 bytes):
///   [0]       discriminator (u8) -- external, trait-provided
///   [1]       version (u8) -- external, trait-provided
///   [2]       bump (u8)
///   [3..11]   max_delay (i64, little-endian, seconds)
///   [11..43]  permission_manager program ID (Address / 32 bytes)
///
/// Note: repr(C) with [u8;8] fields avoids alignment padding.
#[derive(Clone, Debug, PartialEq, CodamaAccount)]
#[codama(field("discriminator", number(u8), default_value = 1))]
#[codama(field("version", number(u8), default_value = 1))]
#[codama(discriminator(field = "discriminator"))]
#[codama(seed(type = string(utf8), value = "minter_config"))]
#[repr(C)]
pub struct MinterConfig {
    pub bump: u8,
    #[codama(type = number(i64))]
    max_delay: [u8; 8],
    pub permission_manager: Address,
}

assert_no_padding!(MinterConfig, 1 + 8 + 32);

impl Discriminator for MinterConfig {
    const DISCRIMINATOR: u8 = DISCRIMINATOR_MINTER_CONFIG;
}

impl Versioned for MinterConfig {
    const VERSION: u8 = 1;
}

impl AccountSize for MinterConfig {
    const DATA_LEN: usize = 1 + 8 + 32; // bump + max_delay + permission_manager
}

impl AccountDeserialize for MinterConfig {}

impl PdaSeeds for MinterConfig {
    const PREFIX: &'static [u8] = MINTER_CONFIG_SEED;

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

impl PdaAccount for MinterConfig {
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

impl MinterConfig {
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
/// On-chain layout (total: 27 bytes):
///   [0]       discriminator (u8) -- external, trait-provided
///   [1]       version (u8) -- external, trait-provided
///   [2]       bump (u8)
///   [3..11]   limit (u64, little-endian)
///   [11..19]  used_amount (u64, little-endian)
///   [19..27]  last_day (i64, little-endian) -- floor(timestamp / 86400)
#[derive(Clone, Debug, PartialEq, CodamaAccount)]
#[codama(field("discriminator", number(u8), default_value = 2))]
#[codama(field("version", number(u8), default_value = 1))]
#[codama(discriminator(field = "discriminator"))]
#[codama(seed(type = string(utf8), value = "daily_limit"))]
#[codama(seed(name = "mint", type = public_key))]
#[repr(C)]
pub struct DailyLimit {
    pub bump: u8,
    #[codama(type = number(u64))]
    limit: [u8; 8],
    #[codama(type = number(u64))]
    used_amount: [u8; 8],
    #[codama(type = number(i64))]
    last_day: [u8; 8],
}

assert_no_padding!(DailyLimit, 1 + 8 + 8 + 8);

impl Discriminator for DailyLimit {
    const DISCRIMINATOR: u8 = DISCRIMINATOR_DAILY_LIMIT;
}

impl Versioned for DailyLimit {
    const VERSION: u8 = 1;
}

impl AccountSize for DailyLimit {
    const DATA_LEN: usize = 1 + 8 + 8 + 8; // bump + limit + used_amount + last_day
}

impl AccountDeserialize for DailyLimit {}

impl PdaSeeds for DailyLimit {
    const PREFIX: &'static [u8] = DAILY_LIMIT_SEED;

    fn validate_pda_address(
        &self,
        account: &AccountView,
        program_id: &Address,
    ) -> Result<u8, ProgramError> {
        // DailyLimit requires the mint key as additional seed, but we don't store it.
        // Validation must be done externally with verify_pda.
        let _ = (account, program_id);
        Err(ProgramError::InvalidSeeds)
    }
}

impl PdaAccount for DailyLimit {
    fn bump(&self) -> u8 {
        self.bump
    }

    fn validate_self(
        &self,
        account: &AccountView,
        program_id: &Address,
    ) -> Result<(), ProgramError> {
        // DailyLimit requires the mint key as additional seed, but we don't store it.
        // Validation must be done externally with verify_pda.
        let _ = (account, program_id);
        Err(ProgramError::InvalidSeeds)
    }
}

impl DailyLimit {
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
/// On-chain layout (total: 12 bytes):
///   [0]       discriminator (u8) -- external, trait-provided
///   [1]       version (u8) -- external, trait-provided
///   [2]       bump (u8)
///   [3]       status (u8): NULL=0, PENDING=1, DONE=2
///   [4..12]   deadline (i64, little-endian, unix timestamp)
#[derive(Clone, Debug, PartialEq, CodamaAccount)]
#[codama(field("discriminator", number(u8), default_value = 3))]
#[codama(field("version", number(u8), default_value = 1))]
#[codama(discriminator(field = "discriminator"))]
#[codama(seed(type = string(utf8), value = "mint_op"))]
#[codama(seed(name = "operationId", type = bytes))]
#[repr(C, packed)]
pub struct MintOperation {
    pub bump: u8,
    pub status: u8,
    #[codama(type = number(i64))]
    deadline: [u8; 8],
}

assert_no_padding!(MintOperation, 1 + 1 + 8);

impl Discriminator for MintOperation {
    const DISCRIMINATOR: u8 = DISCRIMINATOR_MINT_OPERATION;
}

impl Versioned for MintOperation {
    const VERSION: u8 = 1;
}

impl AccountSize for MintOperation {
    const DATA_LEN: usize = 1 + 1 + 8; // bump + status + deadline
}

impl AccountDeserialize for MintOperation {}

impl PdaSeeds for MintOperation {
    const PREFIX: &'static [u8] = MINT_OPERATION_SEED;

    fn validate_pda_address(
        &self,
        account: &AccountView,
        program_id: &Address,
    ) -> Result<u8, ProgramError> {
        // MintOperation requires the operation_id as additional seed, but we don't store it.
        // Validation must be done externally with verify_pda.
        let _ = (account, program_id);
        Err(ProgramError::InvalidSeeds)
    }
}

impl PdaAccount for MintOperation {
    fn bump(&self) -> u8 {
        self.bump
    }

    fn validate_self(
        &self,
        account: &AccountView,
        program_id: &Address,
    ) -> Result<(), ProgramError> {
        // MintOperation requires the operation_id as additional seed, but we don't store it.
        // Validation must be done externally with verify_pda.
        let _ = (account, program_id);
        Err(ProgramError::InvalidSeeds)
    }
}

impl MintOperation {
    pub fn deadline(&self) -> i64 {
        i64::from_le_bytes(self.deadline)
    }

    pub fn set_deadline(&mut self, value: i64) {
        self.deadline = value.to_le_bytes();
    }
}
