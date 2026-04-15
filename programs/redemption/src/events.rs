//! Structured events for the Redemption program.

extern crate alloc;

use alloc::vec::Vec;
use spiko_events::{build_event_data, push_address, push_i64, push_u64};

// SHA256("event:RedemptionInitialized")[0..8]
const DISC_REDEMPTION_INITIALIZED: [u8; 8] = [0x6a, 0xc8, 0x64, 0x72, 0x94, 0x64, 0x26, 0xcb];
// SHA256("event:RedemptionInitiated")[0..8]
const DISC_REDEMPTION_INITIATED: [u8; 8] = [0x55, 0xfe, 0xeb, 0x0e, 0xdd, 0x88, 0x60, 0xde];
// SHA256("event:RedemptionExecuted")[0..8]
const DISC_REDEMPTION_EXECUTED: [u8; 8] = [0xae, 0xda, 0x05, 0x38, 0x24, 0x2e, 0x35, 0xd4];
// SHA256("event:RedemptionCanceled")[0..8]
const DISC_REDEMPTION_CANCELED: [u8; 8] = [0xbd, 0xf4, 0xd0, 0xe8, 0x3c, 0x68, 0xe7, 0xa4];
// SHA256("event:TokenMinimumUpdated")[0..8]
const DISC_TOKEN_MINIMUM_UPDATED: [u8; 8] = [0xeb, 0x3c, 0x99, 0x47, 0x61, 0xd4, 0x70, 0x6e];

/// Build `RedemptionInitialized` event data.
/// Fields: admin (32)
#[inline]
pub fn build_redemption_initialized_event(admin: &[u8; 32]) -> Vec<u8> {
    let mut data = build_event_data(&DISC_REDEMPTION_INITIALIZED, 32);
    push_address(&mut data, admin);
    data
}

/// Build `RedemptionInitiated` event data.
/// Fields: user (32) + mint (32) + amount (8) + salt (8) + deadline (8)
#[inline]
pub fn build_redemption_initiated_event(
    user: &[u8; 32],
    mint: &[u8; 32],
    amount: u64,
    salt: u64,
    deadline: i64,
) -> Vec<u8> {
    let mut data = build_event_data(&DISC_REDEMPTION_INITIATED, 88);
    push_address(&mut data, user);
    push_address(&mut data, mint);
    push_u64(&mut data, amount);
    push_u64(&mut data, salt);
    push_i64(&mut data, deadline);
    data
}

/// Build `RedemptionExecuted` event data.
/// Fields: operator (32) + user (32) + mint (32) + amount (8) + salt (8)
#[inline]
pub fn build_redemption_executed_event(
    operator: &[u8; 32],
    user: &[u8; 32],
    mint: &[u8; 32],
    amount: u64,
    salt: u64,
) -> Vec<u8> {
    let mut data = build_event_data(&DISC_REDEMPTION_EXECUTED, 112);
    push_address(&mut data, operator);
    push_address(&mut data, user);
    push_address(&mut data, mint);
    push_u64(&mut data, amount);
    push_u64(&mut data, salt);
    data
}

/// Build `RedemptionCanceled` event data.
/// Fields: caller (32) + user (32) + mint (32) + amount (8) + salt (8)
#[inline]
pub fn build_redemption_canceled_event(
    caller: &[u8; 32],
    user: &[u8; 32],
    mint: &[u8; 32],
    amount: u64,
    salt: u64,
) -> Vec<u8> {
    let mut data = build_event_data(&DISC_REDEMPTION_CANCELED, 112);
    push_address(&mut data, caller);
    push_address(&mut data, user);
    push_address(&mut data, mint);
    push_u64(&mut data, amount);
    push_u64(&mut data, salt);
    data
}

/// Build `TokenMinimumUpdated` event data.
/// Fields: caller (32) + mint (32) + minimum (8)
#[inline]
pub fn build_token_minimum_updated_event(
    caller: &[u8; 32],
    mint: &[u8; 32],
    minimum: u64,
) -> Vec<u8> {
    let mut data = build_event_data(&DISC_TOKEN_MINIMUM_UPDATED, 72);
    push_address(&mut data, caller);
    push_address(&mut data, mint);
    push_u64(&mut data, minimum);
    data
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    fn expected_disc(event_name: &str) -> [u8; 8] {
        let hash = Sha256::digest(event_name.as_bytes());
        hash[..8].try_into().unwrap()
    }

    #[test]
    fn verify_event_discriminators() {
        assert_eq!(
            DISC_REDEMPTION_INITIALIZED,
            expected_disc("event:RedemptionInitialized")
        );
        assert_eq!(
            DISC_REDEMPTION_INITIATED,
            expected_disc("event:RedemptionInitiated")
        );
        assert_eq!(
            DISC_REDEMPTION_EXECUTED,
            expected_disc("event:RedemptionExecuted")
        );
        assert_eq!(
            DISC_REDEMPTION_CANCELED,
            expected_disc("event:RedemptionCanceled")
        );
        assert_eq!(
            DISC_TOKEN_MINIMUM_UPDATED,
            expected_disc("event:TokenMinimumUpdated")
        );
    }
}
