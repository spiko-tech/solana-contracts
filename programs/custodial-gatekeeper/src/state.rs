use codama::CodamaAccount;
use pinocchio::{account::AccountView, address::Address, error::ProgramError};

use spiko_common::{
    assert_no_padding, AccountDeserialize, AccountSize, Discriminator, PdaAccount, PdaSeeds,
    Versioned,
};

pub const GATEKEEPER_CONFIG_SEED: &[u8] = b"gatekeeper_config";
pub const WITHDRAWAL_DAILY_LIMIT_SEED: &[u8] = b"withdrawal_limit";
pub const WITHDRAWAL_OPERATION_SEED: &[u8] = b"withdrawal_op";
pub const VAULT_SEED: &[u8] = b"vault";

pub const DISCRIMINATOR_GATEKEEPER_CONFIG: u8 = 1;
pub const DISCRIMINATOR_WITHDRAWAL_DAILY_LIMIT: u8 = 2;
pub const DISCRIMINATOR_WITHDRAWAL_OPERATION: u8 = 3;

pub const STATUS_NULL: u8 = 0;
pub const STATUS_PENDING: u8 = 1;
pub const STATUS_DONE: u8 = 2;
pub const STATUS_CANCELED: u8 = 3;

pub const SECONDS_PER_DAY: i64 = 86_400;

/// Global configuration for the CustodialGatekeeper program.
///
/// Seeds: ["gatekeeper_config"]
///
/// On-chain layout (total: 43 bytes):
///   [0]       discriminator (u8) -- external, trait-provided
///   [1]       version (u8) -- external, trait-provided
///   [2]       bump (u8)
///   [3..11]   max_delay (i64, little-endian, seconds)
///   [11..43]  permission_manager program ID (Address / 32 bytes)
#[derive(Clone, Debug, PartialEq, CodamaAccount)]
#[codama(field("discriminator", number(u8), default_value = 1))]
#[codama(field("version", number(u8), default_value = 1))]
#[codama(discriminator(field = "discriminator"))]
#[codama(seed(type = string(utf8), value = "gatekeeper_config"))]
#[repr(C)]
pub struct GatekeeperConfig {
    pub bump: u8,
    #[codama(type = number(i64))]
    max_delay: [u8; 8],
    pub permission_manager: Address,
}

assert_no_padding!(GatekeeperConfig, 1 + 8 + 32);

impl Discriminator for GatekeeperConfig {
    const DISCRIMINATOR: u8 = DISCRIMINATOR_GATEKEEPER_CONFIG;
}

impl Versioned for GatekeeperConfig {
    const VERSION: u8 = 1;
}

impl AccountSize for GatekeeperConfig {
    const DATA_LEN: usize = 1 + 8 + 32; // bump + max_delay + permission_manager
}

impl AccountDeserialize for GatekeeperConfig {}

impl PdaSeeds for GatekeeperConfig {
    const PREFIX: &'static [u8] = GATEKEEPER_CONFIG_SEED;

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

impl PdaAccount for GatekeeperConfig {
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

impl GatekeeperConfig {
    pub fn max_delay(&self) -> i64 {
        i64::from_le_bytes(self.max_delay)
    }

    pub fn set_max_delay(&mut self, value: i64) {
        self.max_delay = value.to_le_bytes();
    }
}

/// Tracks the daily withdrawal limit and usage for a specific token.
///
/// Seeds: ["withdrawal_limit", mint_pubkey]
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
#[codama(seed(type = string(utf8), value = "withdrawal_limit"))]
#[codama(seed(name = "mint", type = public_key))]
#[repr(C)]
pub struct WithdrawalDailyLimit {
    pub bump: u8,
    #[codama(type = number(u64))]
    limit: [u8; 8],
    #[codama(type = number(u64))]
    used_amount: [u8; 8],
    #[codama(type = number(i64))]
    last_day: [u8; 8],
}

assert_no_padding!(WithdrawalDailyLimit, 1 + 8 + 8 + 8);

impl Discriminator for WithdrawalDailyLimit {
    const DISCRIMINATOR: u8 = DISCRIMINATOR_WITHDRAWAL_DAILY_LIMIT;
}

impl Versioned for WithdrawalDailyLimit {
    const VERSION: u8 = 1;
}

impl AccountSize for WithdrawalDailyLimit {
    const DATA_LEN: usize = 1 + 8 + 8 + 8; // bump + limit + used_amount + last_day
}

impl AccountDeserialize for WithdrawalDailyLimit {}

impl WithdrawalDailyLimit {
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

/// Tracks the status of a single withdrawal operation.
///
/// Seeds: ["withdrawal_op", operation_id (32 bytes)]
///
/// On-chain layout (total: 77 bytes):
///   [0]       discriminator (u8) -- external, trait-provided
///   [1]       version (u8) -- external, trait-provided
///   [2]       bump (u8)
///   [3]       status (u8): NULL=0, PENDING=1, DONE=2, CANCELED=3
///   [4..12]   deadline (i64, little-endian, unix timestamp)
///   [12..44]  recipient address (Address / 32 bytes)
///   [44..76]  mint address (Address / 32 bytes)
///   [76..84]  amount (u64, little-endian)
#[derive(Clone, Debug, PartialEq, CodamaAccount)]
#[codama(field("discriminator", number(u8), default_value = 3))]
#[codama(field("version", number(u8), default_value = 1))]
#[codama(discriminator(field = "discriminator"))]
#[codama(seed(type = string(utf8), value = "withdrawal_op"))]
#[codama(seed(name = "operationId", type = bytes))]
#[repr(C)]
pub struct WithdrawalOperation {
    pub bump: u8,
    pub status: u8,
    #[codama(type = number(i64))]
    deadline: [u8; 8],
    #[codama(type = public_key)]
    pub recipient: Address,
    #[codama(type = public_key)]
    pub mint: Address,
    #[codama(type = number(u64))]
    amount: [u8; 8],
}

assert_no_padding!(WithdrawalOperation, 1 + 1 + 8 + 32 + 32 + 8);

impl Discriminator for WithdrawalOperation {
    const DISCRIMINATOR: u8 = DISCRIMINATOR_WITHDRAWAL_OPERATION;
}

impl Versioned for WithdrawalOperation {
    const VERSION: u8 = 1;
}

impl AccountSize for WithdrawalOperation {
    const DATA_LEN: usize = 1 + 1 + 8 + 32 + 32 + 8;
}

impl AccountDeserialize for WithdrawalOperation {}

impl WithdrawalOperation {
    pub fn deadline(&self) -> i64 {
        i64::from_le_bytes(self.deadline)
    }

    pub fn set_deadline(&mut self, value: i64) {
        self.deadline = value.to_le_bytes();
    }

    pub fn amount(&self) -> u64 {
        u64::from_le_bytes(self.amount)
    }

    pub fn set_amount(&mut self, value: u64) {
        self.amount = value.to_le_bytes();
    }
}
