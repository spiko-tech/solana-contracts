//! Structured events for the Redemption program.
//!
//! Each function emits an Anchor-compatible event via `sol_log_data`:
//! discriminator (8 bytes) = SHA256("event:<EventName>")[0..8], then LE-packed fields.

use spiko_events::{emit_event, pack_address, pack_disc, pack_i64, pack_u64};

// ---------------------------------------------------------------
// Discriminators — precomputed SHA256("event:<EventName>")[0..8]
// ---------------------------------------------------------------

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

// ---------------------------------------------------------------
// Event emitters
// ---------------------------------------------------------------

/// Emit `RedemptionInitialized { admin: [u8;32] }`
/// Buffer: disc(8) + admin(32) = 40 bytes
#[inline]
pub fn emit_redemption_initialized(admin: &[u8; 32]) {
    let mut buf = [0u8; 40];
    let off = pack_disc(&mut buf, &DISC_REDEMPTION_INITIALIZED);
    pack_address(&mut buf, off, admin);
    emit_event(&buf);
}

/// Emit `RedemptionInitiated { user: [u8;32], mint: [u8;32], amount: u64, salt: u64, deadline: i64 }`
/// Buffer: disc(8) + user(32) + mint(32) + amount(8) + salt(8) + deadline(8) = 96 bytes
#[inline]
pub fn emit_redemption_initiated(
    user: &[u8; 32],
    mint: &[u8; 32],
    amount: u64,
    salt: u64,
    deadline: i64,
) {
    let mut buf = [0u8; 96];
    let off = pack_disc(&mut buf, &DISC_REDEMPTION_INITIATED);
    let off = pack_address(&mut buf, off, user);
    let off = pack_address(&mut buf, off, mint);
    let off = pack_u64(&mut buf, off, amount);
    let off = pack_u64(&mut buf, off, salt);
    pack_i64(&mut buf, off, deadline);
    emit_event(&buf);
}

/// Emit `RedemptionExecuted { operator: [u8;32], user: [u8;32], mint: [u8;32], amount: u64, salt: u64 }`
/// Buffer: disc(8) + operator(32) + user(32) + mint(32) + amount(8) + salt(8) = 120 bytes
#[inline]
pub fn emit_redemption_executed(
    operator: &[u8; 32],
    user: &[u8; 32],
    mint: &[u8; 32],
    amount: u64,
    salt: u64,
) {
    let mut buf = [0u8; 120];
    let off = pack_disc(&mut buf, &DISC_REDEMPTION_EXECUTED);
    let off = pack_address(&mut buf, off, operator);
    let off = pack_address(&mut buf, off, user);
    let off = pack_address(&mut buf, off, mint);
    let off = pack_u64(&mut buf, off, amount);
    pack_u64(&mut buf, off, salt);
    emit_event(&buf);
}

/// Emit `RedemptionCanceled { caller: [u8;32], user: [u8;32], mint: [u8;32], amount: u64, salt: u64 }`
/// Buffer: disc(8) + caller(32) + user(32) + mint(32) + amount(8) + salt(8) = 120 bytes
#[inline]
pub fn emit_redemption_canceled(
    caller: &[u8; 32],
    user: &[u8; 32],
    mint: &[u8; 32],
    amount: u64,
    salt: u64,
) {
    let mut buf = [0u8; 120];
    let off = pack_disc(&mut buf, &DISC_REDEMPTION_CANCELED);
    let off = pack_address(&mut buf, off, caller);
    let off = pack_address(&mut buf, off, user);
    let off = pack_address(&mut buf, off, mint);
    let off = pack_u64(&mut buf, off, amount);
    pack_u64(&mut buf, off, salt);
    emit_event(&buf);
}

/// Emit `TokenMinimumUpdated { caller: [u8;32], mint: [u8;32], minimum: u64 }`
/// Buffer: disc(8) + caller(32) + mint(32) + minimum(8) = 80 bytes
#[inline]
pub fn emit_token_minimum_updated(caller: &[u8; 32], mint: &[u8; 32], minimum: u64) {
    let mut buf = [0u8; 80];
    let off = pack_disc(&mut buf, &DISC_TOKEN_MINIMUM_UPDATED);
    let off = pack_address(&mut buf, off, caller);
    let off = pack_address(&mut buf, off, mint);
    pack_u64(&mut buf, off, minimum);
    emit_event(&buf);
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
