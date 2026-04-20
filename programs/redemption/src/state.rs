use codama::CodamaAccount;
use pinocchio::{account::AccountView, address::Address, error::ProgramError};

use spiko_common::{
    assert_no_padding, AccountDeserialize, AccountSize, Discriminator, PdaAccount, PdaSeeds,
    Versioned,
};

pub const REDEMPTION_CONFIG_SEED: &[u8] = b"redemption_config";
pub const TOKEN_MINIMUM_SEED: &[u8] = b"minimum";
pub const REDEMPTION_OPERATION_SEED: &[u8] = b"redemption_op";
pub const VAULT_SEED: &[u8] = b"vault";

pub const DISCRIMINATOR_REDEMPTION_CONFIG: u8 = 1;
pub const DISCRIMINATOR_TOKEN_MINIMUM: u8 = 2;
pub const DISCRIMINATOR_REDEMPTION_OPERATION: u8 = 4;

pub const STATUS_NULL: u8 = 0;
pub const STATUS_PENDING: u8 = 1;
pub const STATUS_EXECUTED: u8 = 2;
pub const STATUS_CANCELED: u8 = 3;

/// Maximum delay for redemption operations (14 days in seconds).
pub const MAX_DELAY: i64 = 14 * 24 * 60 * 60; // 1_209_600

/// Global configuration for the Redemption program.
///
/// Seeds: ["redemption_config"]
///
/// On-chain layout (total: 35 bytes):
///   [0]       discriminator (u8) -- external, trait-provided
///   [1]       version (u8) -- external, trait-provided
///   [2]       bump (u8)
///   [3..35]   permission_manager program ID (Address / 32 bytes)
#[derive(Clone, Debug, PartialEq, CodamaAccount)]
#[codama(field("discriminator", number(u8), default_value = 1))]
#[codama(field("version", number(u8), default_value = 1))]
#[codama(discriminator(field = "discriminator"))]
#[codama(seed(type = string(utf8), value = "redemption_config"))]
#[repr(C)]
pub struct RedemptionConfig {
    pub bump: u8,
    #[codama(type = public_key)]
    pub permission_manager: Address,
}

assert_no_padding!(RedemptionConfig, 1 + 32);

impl Discriminator for RedemptionConfig {
    const DISCRIMINATOR: u8 = DISCRIMINATOR_REDEMPTION_CONFIG;
}

impl Versioned for RedemptionConfig {
    const VERSION: u8 = 1;
}

impl AccountSize for RedemptionConfig {
    const DATA_LEN: usize = 1 + 32; // bump + permission_manager
}

impl AccountDeserialize for RedemptionConfig {}

impl PdaSeeds for RedemptionConfig {
    const PREFIX: &'static [u8] = REDEMPTION_CONFIG_SEED;

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

impl PdaAccount for RedemptionConfig {
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

/// Stores the minimum redemption amount for a specific token.
///
/// Seeds: ["minimum", mint_pubkey]
///
/// On-chain layout (total: 11 bytes):
///   [0]       discriminator (u8) -- external, trait-provided
///   [1]       version (u8) -- external, trait-provided
///   [2]       bump (u8)
///   [3..11]   minimum_amount (u64, little-endian)
#[derive(Clone, Debug, PartialEq, CodamaAccount)]
#[codama(field("discriminator", number(u8), default_value = 2))]
#[codama(field("version", number(u8), default_value = 1))]
#[codama(discriminator(field = "discriminator"))]
#[codama(seed(type = string(utf8), value = "minimum"))]
#[codama(seed(name = "mint", type = public_key))]
#[repr(C)]
pub struct TokenMinimum {
    pub bump: u8,
    #[codama(type = number(u64))]
    minimum_amount: [u8; 8],
}

assert_no_padding!(TokenMinimum, 1 + 8);

impl Discriminator for TokenMinimum {
    const DISCRIMINATOR: u8 = DISCRIMINATOR_TOKEN_MINIMUM;
}

impl Versioned for TokenMinimum {
    const VERSION: u8 = 1;
}

impl AccountSize for TokenMinimum {
    const DATA_LEN: usize = 1 + 8; // bump + minimum_amount
}

impl AccountDeserialize for TokenMinimum {}

impl TokenMinimum {
    pub fn minimum_amount(&self) -> u64 {
        u64::from_le_bytes(self.minimum_amount)
    }

    pub fn set_minimum_amount(&mut self, value: u64) {
        self.minimum_amount = value.to_le_bytes();
    }
}

/// Tracks the status of a single redemption operation.
///
/// Seeds: ["redemption_op", operation_id (32 bytes)]
///
/// On-chain layout (total: 45 bytes):
///   [0]       discriminator (u8) -- external, trait-provided
///   [1]       version (u8) -- external, trait-provided
///   [2]       bump (u8)
///   [3]       status (u8): NULL=0, PENDING=1, EXECUTED=2, CANCELED=3
///   [4]       _padding (u8)
///   [5..13]   deadline (i64, little-endian, unix timestamp)
///   [13..45]  user address (Address / 32 bytes, for refund on cancel)
#[derive(Clone, Debug, PartialEq, CodamaAccount)]
#[codama(field("discriminator", number(u8), default_value = 4))]
#[codama(field("version", number(u8), default_value = 1))]
#[codama(discriminator(field = "discriminator"))]
#[codama(seed(type = string(utf8), value = "redemption_op"))]
#[codama(seed(name = "operationId", type = bytes))]
#[repr(C)]
pub struct RedemptionOperation {
    pub bump: u8,
    pub status: u8,
    _padding: u8,
    #[codama(type = number(i64))]
    deadline: [u8; 8],
    #[codama(type = public_key)]
    pub user: Address,
}

assert_no_padding!(RedemptionOperation, 1 + 1 + 1 + 8 + 32);

impl Discriminator for RedemptionOperation {
    const DISCRIMINATOR: u8 = DISCRIMINATOR_REDEMPTION_OPERATION;
}

impl Versioned for RedemptionOperation {
    const VERSION: u8 = 1;
}

impl AccountSize for RedemptionOperation {
    const DATA_LEN: usize = 1 + 1 + 1 + 8 + 32; // bump + status + _padding + deadline + user
}

impl AccountDeserialize for RedemptionOperation {}

impl RedemptionOperation {
    pub fn deadline(&self) -> i64 {
        i64::from_le_bytes(self.deadline)
    }

    pub fn set_deadline(&mut self, value: i64) {
        self.deadline = value.to_le_bytes();
    }
}
