//! Structured events for the Spiko Token program.
//!
//! Each function builds an Anchor-compatible CPI event payload:
//! `EVENT_IX_TAG (8) + discriminator (8) + LE-packed fields`.

extern crate alloc;

use alloc::vec::Vec;
use spiko_events::{build_event_data, push_address, push_u64};

// SHA256("event:TokenInitialized")[0..8]
const DISC_TOKEN_INITIALIZED: [u8; 8] = [0x4d, 0x46, 0xe9, 0x7c, 0xec, 0x5c, 0xcc, 0x00];
// SHA256("event:Mint")[0..8]
const DISC_MINT: [u8; 8] = [0x3f, 0x0b, 0xd5, 0x86, 0x94, 0xc2, 0x18, 0xcb];
// SHA256("event:Burn")[0..8]
const DISC_BURN: [u8; 8] = [0xb8, 0x0d, 0x41, 0xce, 0xce, 0xaa, 0x33, 0x55];
// SHA256("event:RedeemInitiated")[0..8]
const DISC_REDEEM_INITIATED: [u8; 8] = [0x47, 0xdc, 0x92, 0xb9, 0x0b, 0xdc, 0xf5, 0x13];
// SHA256("event:TokenPaused")[0..8]
const DISC_TOKEN_PAUSED: [u8; 8] = [0x7e, 0x36, 0x4c, 0xa1, 0x7d, 0x97, 0x94, 0x3b];
// SHA256("event:TokenUnpaused")[0..8]
const DISC_TOKEN_UNPAUSED: [u8; 8] = [0xe1, 0x11, 0x44, 0x51, 0x81, 0x86, 0x91, 0xa9];
// SHA256("event:RedemptionContractSet")[0..8]
const DISC_REDEMPTION_CONTRACT_SET: [u8; 8] = [0xbd, 0xb3, 0x1c, 0x22, 0xe3, 0x63, 0xf6, 0x3a];

/// Build `TokenInitialized` event data.
/// Fields: admin (32) + mint (32)
#[inline]
pub fn build_token_initialized_event(admin: &[u8; 32], mint: &[u8; 32]) -> Vec<u8> {
    let mut data = build_event_data(&DISC_TOKEN_INITIALIZED, 64);
    push_address(&mut data, admin);
    push_address(&mut data, mint);
    data
}

/// Build `Mint` event data.
/// Fields: caller (32) + mint (32) + recipient_ata (32) + amount (8)
#[inline]
pub fn build_mint_event(
    caller: &[u8; 32],
    mint: &[u8; 32],
    recipient_ata: &[u8; 32],
    amount: u64,
) -> Vec<u8> {
    let mut data = build_event_data(&DISC_MINT, 104);
    push_address(&mut data, caller);
    push_address(&mut data, mint);
    push_address(&mut data, recipient_ata);
    push_u64(&mut data, amount);
    data
}

/// Build `Burn` event data.
/// Fields: caller (32) + mint (32) + source_ata (32) + amount (8)
#[inline]
pub fn build_burn_event(
    caller: &[u8; 32],
    mint: &[u8; 32],
    source_ata: &[u8; 32],
    amount: u64,
) -> Vec<u8> {
    let mut data = build_event_data(&DISC_BURN, 104);
    push_address(&mut data, caller);
    push_address(&mut data, mint);
    push_address(&mut data, source_ata);
    push_u64(&mut data, amount);
    data
}

/// Build `RedeemInitiated` event data.
/// Fields: user (32) + mint (32) + amount (8) + salt (8)
#[inline]
pub fn build_redeem_initiated_event(
    user: &[u8; 32],
    mint: &[u8; 32],
    amount: u64,
    salt: u64,
) -> Vec<u8> {
    let mut data = build_event_data(&DISC_REDEEM_INITIATED, 80);
    push_address(&mut data, user);
    push_address(&mut data, mint);
    push_u64(&mut data, amount);
    push_u64(&mut data, salt);
    data
}

/// Build `TokenPaused` event data.
/// Fields: caller (32) + config (32)
#[inline]
pub fn build_token_paused_event(caller: &[u8; 32], config: &[u8; 32]) -> Vec<u8> {
    let mut data = build_event_data(&DISC_TOKEN_PAUSED, 64);
    push_address(&mut data, caller);
    push_address(&mut data, config);
    data
}

/// Build `TokenUnpaused` event data.
/// Fields: caller (32) + config (32)
#[inline]
pub fn build_token_unpaused_event(caller: &[u8; 32], config: &[u8; 32]) -> Vec<u8> {
    let mut data = build_event_data(&DISC_TOKEN_UNPAUSED, 64);
    push_address(&mut data, caller);
    push_address(&mut data, config);
    data
}

/// Build `RedemptionContractSet` event data.
/// Fields: caller (32) + config (32) + contract (32)
#[inline]
pub fn build_redemption_contract_set_event(
    caller: &[u8; 32],
    config: &[u8; 32],
    contract: &[u8; 32],
) -> Vec<u8> {
    let mut data = build_event_data(&DISC_REDEMPTION_CONTRACT_SET, 96);
    push_address(&mut data, caller);
    push_address(&mut data, config);
    push_address(&mut data, contract);
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
            DISC_TOKEN_INITIALIZED,
            expected_disc("event:TokenInitialized")
        );
        assert_eq!(DISC_MINT, expected_disc("event:Mint"));
        assert_eq!(DISC_BURN, expected_disc("event:Burn"));
        assert_eq!(
            DISC_REDEEM_INITIATED,
            expected_disc("event:RedeemInitiated")
        );
        assert_eq!(DISC_TOKEN_PAUSED, expected_disc("event:TokenPaused"));
        assert_eq!(DISC_TOKEN_UNPAUSED, expected_disc("event:TokenUnpaused"));
        assert_eq!(
            DISC_REDEMPTION_CONTRACT_SET,
            expected_disc("event:RedemptionContractSet")
        );
    }
}
