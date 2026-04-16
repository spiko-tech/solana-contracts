//! Codama event type definitions for the Spiko Transfer Hook program.

use codama::CodamaType;
use pinocchio::Address;

/// Emitted on each transfer hook execution.
#[derive(CodamaType)]
pub struct TransferEvent {
    pub sender: Address,
    pub mint: Address,
    pub source: Address,
    pub destination: Address,
    pub amount: u64,
}
