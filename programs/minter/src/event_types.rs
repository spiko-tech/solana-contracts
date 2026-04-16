//! Codama event type definitions for the Minter program.

use codama::CodamaType;
use pinocchio::Address;

#[derive(CodamaType)]
pub struct MinterInitializedEvent {
    pub admin: Address,
}

#[derive(CodamaType)]
pub struct MintExecutedEvent {
    pub caller: Address,
    pub user: Address,
    pub mint: Address,
    pub amount: u64,
    pub salt: u64,
}

#[derive(CodamaType)]
pub struct MintBlockedEvent {
    pub caller: Address,
    pub user: Address,
    pub mint: Address,
    pub amount: u64,
    pub salt: u64,
}

#[derive(CodamaType)]
pub struct MintApprovedEvent {
    pub caller: Address,
    pub user: Address,
    pub mint: Address,
    pub amount: u64,
    pub salt: u64,
}

#[derive(CodamaType)]
pub struct MintCanceledEvent {
    pub caller: Address,
    pub user: Address,
    pub mint: Address,
    pub amount: u64,
    pub salt: u64,
}

#[derive(CodamaType)]
pub struct DailyLimitUpdatedEvent {
    pub caller: Address,
    pub mint: Address,
    pub limit: u64,
}

#[derive(CodamaType)]
pub struct MaxDelayUpdatedEvent {
    pub caller: Address,
    pub max_delay: i64,
}
