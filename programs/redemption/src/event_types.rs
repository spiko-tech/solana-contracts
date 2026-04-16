//! Codama event type definitions for the Redemption program.

use codama::CodamaType;
use pinocchio::Address;

#[derive(CodamaType)]
pub struct RedemptionInitializedEvent {
    pub admin: Address,
}

#[derive(CodamaType)]
pub struct RedemptionInitiatedEvent {
    pub user: Address,
    pub mint: Address,
    pub amount: u64,
    pub salt: u64,
    pub deadline: i64,
}

#[derive(CodamaType)]
pub struct RedemptionExecutedEvent {
    pub operator: Address,
    pub user: Address,
    pub mint: Address,
    pub amount: u64,
    pub salt: u64,
}

#[derive(CodamaType)]
pub struct RedemptionCanceledEvent {
    pub caller: Address,
    pub user: Address,
    pub mint: Address,
    pub amount: u64,
    pub salt: u64,
}

#[derive(CodamaType)]
pub struct TokenMinimumUpdatedEvent {
    pub caller: Address,
    pub mint: Address,
    pub minimum: u64,
}
