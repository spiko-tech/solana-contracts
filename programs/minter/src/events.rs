//! Structured events for the Minter program.

extern crate alloc;

use alloc::vec::Vec;
use spiko_events::{build_event_data, push_address, push_i64, push_u64};

// SHA256("event:MinterInitialized")[0..8]
const DISC_MINTER_INITIALIZED: [u8; 8] = [0xb1, 0x89, 0x62, 0xb3, 0x16, 0xce, 0x37, 0xc0];
// SHA256("event:MintExecuted")[0..8]
const DISC_MINT_EXECUTED: [u8; 8] = [0x37, 0x87, 0x6c, 0x49, 0x05, 0xbe, 0xed, 0x2c];
// SHA256("event:MintBlocked")[0..8]
const DISC_MINT_BLOCKED: [u8; 8] = [0x7e, 0xee, 0x83, 0xcd, 0xfd, 0x6e, 0xf5, 0x23];
// SHA256("event:MintApproved")[0..8]
const DISC_MINT_APPROVED: [u8; 8] = [0x02, 0x44, 0xe9, 0x18, 0x66, 0x41, 0x68, 0x23];
// SHA256("event:MintCanceled")[0..8]
const DISC_MINT_CANCELED: [u8; 8] = [0xa8, 0x4a, 0x13, 0x9d, 0x4a, 0xdd, 0xc0, 0x19];
// SHA256("event:DailyLimitUpdated")[0..8]
const DISC_DAILY_LIMIT_UPDATED: [u8; 8] = [0x41, 0x08, 0xe7, 0xad, 0xd7, 0xb6, 0x47, 0xc9];
// SHA256("event:MaxDelayUpdated")[0..8]
const DISC_MAX_DELAY_UPDATED: [u8; 8] = [0x81, 0x51, 0x91, 0x1a, 0x62, 0xd2, 0xa0, 0x0c];

/// Build `MinterInitialized` event data.
/// Fields: admin (32) + max_delay (8)
#[inline]
pub fn build_minter_initialized_event(admin: &[u8; 32], max_delay: i64) -> Vec<u8> {
    let mut data = build_event_data(&DISC_MINTER_INITIALIZED, 40);
    push_address(&mut data, admin);
    push_i64(&mut data, max_delay);
    data
}

/// Build `MintExecuted` event data.
/// Fields: caller (32) + user (32) + mint (32) + amount (8) + salt (8)
#[inline]
pub fn build_mint_executed_event(
    caller: &[u8; 32],
    user: &[u8; 32],
    mint: &[u8; 32],
    amount: u64,
    salt: u64,
) -> Vec<u8> {
    let mut data = build_event_data(&DISC_MINT_EXECUTED, 112);
    push_address(&mut data, caller);
    push_address(&mut data, user);
    push_address(&mut data, mint);
    push_u64(&mut data, amount);
    push_u64(&mut data, salt);
    data
}

/// Build `MintBlocked` event data.
/// Fields: caller (32) + user (32) + mint (32) + amount (8) + salt (8)
#[inline]
pub fn build_mint_blocked_event(
    caller: &[u8; 32],
    user: &[u8; 32],
    mint: &[u8; 32],
    amount: u64,
    salt: u64,
) -> Vec<u8> {
    let mut data = build_event_data(&DISC_MINT_BLOCKED, 112);
    push_address(&mut data, caller);
    push_address(&mut data, user);
    push_address(&mut data, mint);
    push_u64(&mut data, amount);
    push_u64(&mut data, salt);
    data
}

/// Build `MintApproved` event data.
/// Fields: approver (32) + user (32) + mint (32) + amount (8) + salt (8)
#[inline]
pub fn build_mint_approved_event(
    approver: &[u8; 32],
    user: &[u8; 32],
    mint: &[u8; 32],
    amount: u64,
    salt: u64,
) -> Vec<u8> {
    let mut data = build_event_data(&DISC_MINT_APPROVED, 112);
    push_address(&mut data, approver);
    push_address(&mut data, user);
    push_address(&mut data, mint);
    push_u64(&mut data, amount);
    push_u64(&mut data, salt);
    data
}

/// Build `MintCanceled` event data.
/// Fields: caller (32) + user (32) + mint (32) + amount (8) + salt (8)
#[inline]
pub fn build_mint_canceled_event(
    caller: &[u8; 32],
    user: &[u8; 32],
    mint: &[u8; 32],
    amount: u64,
    salt: u64,
) -> Vec<u8> {
    let mut data = build_event_data(&DISC_MINT_CANCELED, 112);
    push_address(&mut data, caller);
    push_address(&mut data, user);
    push_address(&mut data, mint);
    push_u64(&mut data, amount);
    push_u64(&mut data, salt);
    data
}

/// Build `DailyLimitUpdated` event data.
/// Fields: caller (32) + mint (32) + limit (8)
#[inline]
pub fn build_daily_limit_updated_event(caller: &[u8; 32], mint: &[u8; 32], limit: u64) -> Vec<u8> {
    let mut data = build_event_data(&DISC_DAILY_LIMIT_UPDATED, 72);
    push_address(&mut data, caller);
    push_address(&mut data, mint);
    push_u64(&mut data, limit);
    data
}

/// Build `MaxDelayUpdated` event data.
/// Fields: caller (32) + max_delay (8)
#[inline]
pub fn build_max_delay_updated_event(caller: &[u8; 32], max_delay: i64) -> Vec<u8> {
    let mut data = build_event_data(&DISC_MAX_DELAY_UPDATED, 40);
    push_address(&mut data, caller);
    push_i64(&mut data, max_delay);
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
            DISC_MINTER_INITIALIZED,
            expected_disc("event:MinterInitialized")
        );
        assert_eq!(DISC_MINT_EXECUTED, expected_disc("event:MintExecuted"));
        assert_eq!(DISC_MINT_BLOCKED, expected_disc("event:MintBlocked"));
        assert_eq!(DISC_MINT_APPROVED, expected_disc("event:MintApproved"));
        assert_eq!(DISC_MINT_CANCELED, expected_disc("event:MintCanceled"));
        assert_eq!(
            DISC_DAILY_LIMIT_UPDATED,
            expected_disc("event:DailyLimitUpdated")
        );
        assert_eq!(
            DISC_MAX_DELAY_UPDATED,
            expected_disc("event:MaxDelayUpdated")
        );
    }
}
