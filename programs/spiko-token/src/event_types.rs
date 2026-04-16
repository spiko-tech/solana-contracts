//! Codama event type definitions for the Spiko Token program.

use codama::CodamaType;
use pinocchio::Address;

/// Emitted when a token is initialized.
#[derive(CodamaType)]
pub struct TokenInitializedEvent {
    pub admin: Address,
    pub mint: Address,
}

/// Emitted when tokens are minted.
#[derive(CodamaType)]
pub struct MintEvent {
    pub caller: Address,
    pub mint: Address,
    pub recipient_ata: Address,
    pub amount: u64,
}

/// Emitted when tokens are burned.
#[derive(CodamaType)]
pub struct BurnEvent {
    pub caller: Address,
    pub mint: Address,
    pub source_ata: Address,
    pub amount: u64,
}

/// Emitted when a redemption is initiated.
#[derive(CodamaType)]
pub struct RedeemInitiatedEvent {
    pub user: Address,
    pub mint: Address,
    pub amount: u64,
    pub salt: u64,
}

/// Emitted when a token is paused.
#[derive(CodamaType)]
pub struct TokenPausedEvent {
    pub caller: Address,
    pub config: Address,
}

/// Emitted when a token is unpaused.
#[derive(CodamaType)]
pub struct TokenUnpausedEvent {
    pub caller: Address,
    pub config: Address,
}

/// Emitted when a redemption contract is set.
#[derive(CodamaType)]
pub struct RedemptionContractSetEvent {
    pub caller: Address,
    pub config: Address,
    pub contract: Address,
}
