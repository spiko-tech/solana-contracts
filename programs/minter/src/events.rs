//! Structured events for the Minter program.
//!
//! Each function emits an Anchor-compatible event via `sol_log_data`:
//! discriminator (8 bytes) = SHA256("event:<EventName>")[0..8], then LE-packed fields.

use spiko_events::{emit_event, pack_address, pack_disc, pack_i64, pack_u64};

// ---------------------------------------------------------------
// Discriminators — precomputed SHA256("event:<EventName>")[0..8]
// ---------------------------------------------------------------

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

// ---------------------------------------------------------------
// Event emitters
// ---------------------------------------------------------------

/// Emit `MinterInitialized { admin: [u8;32], max_delay: i64 }`
/// Buffer: disc(8) + admin(32) + max_delay(8) = 48 bytes
#[inline]
pub fn emit_minter_initialized(admin: &[u8; 32], max_delay: i64) {
    pinocchio_log::log!("MinterInitialized");
    let mut buf = [0u8; 48];
    let off = pack_disc(&mut buf, &DISC_MINTER_INITIALIZED);
    let off = pack_address(&mut buf, off, admin);
    pack_i64(&mut buf, off, max_delay);
    emit_event(&buf);
}

/// Emit `MintExecuted { caller: [u8;32], user: [u8;32], mint: [u8;32], amount: u64, salt: u64 }`
/// Buffer: disc(8) + caller(32) + user(32) + mint(32) + amount(8) + salt(8) = 120 bytes
#[inline]
pub fn emit_mint_executed(
    caller: &[u8; 32],
    user: &[u8; 32],
    mint: &[u8; 32],
    amount: u64,
    salt: u64,
) {
    pinocchio_log::log!("MintExecuted");
    let mut buf = [0u8; 120];
    let off = pack_disc(&mut buf, &DISC_MINT_EXECUTED);
    let off = pack_address(&mut buf, off, caller);
    let off = pack_address(&mut buf, off, user);
    let off = pack_address(&mut buf, off, mint);
    let off = pack_u64(&mut buf, off, amount);
    pack_u64(&mut buf, off, salt);
    emit_event(&buf);
}

/// Emit `MintBlocked { caller: [u8;32], user: [u8;32], mint: [u8;32], amount: u64, salt: u64 }`
/// Buffer: disc(8) + caller(32) + user(32) + mint(32) + amount(8) + salt(8) = 120 bytes
#[inline]
pub fn emit_mint_blocked(
    caller: &[u8; 32],
    user: &[u8; 32],
    mint: &[u8; 32],
    amount: u64,
    salt: u64,
) {
    pinocchio_log::log!("MintBlocked");
    let mut buf = [0u8; 120];
    let off = pack_disc(&mut buf, &DISC_MINT_BLOCKED);
    let off = pack_address(&mut buf, off, caller);
    let off = pack_address(&mut buf, off, user);
    let off = pack_address(&mut buf, off, mint);
    let off = pack_u64(&mut buf, off, amount);
    pack_u64(&mut buf, off, salt);
    emit_event(&buf);
}

/// Emit `MintApproved { approver: [u8;32], user: [u8;32], mint: [u8;32], amount: u64, salt: u64 }`
/// Buffer: disc(8) + approver(32) + user(32) + mint(32) + amount(8) + salt(8) = 120 bytes
#[inline]
pub fn emit_mint_approved(
    approver: &[u8; 32],
    user: &[u8; 32],
    mint: &[u8; 32],
    amount: u64,
    salt: u64,
) {
    pinocchio_log::log!("MintApproved");
    let mut buf = [0u8; 120];
    let off = pack_disc(&mut buf, &DISC_MINT_APPROVED);
    let off = pack_address(&mut buf, off, approver);
    let off = pack_address(&mut buf, off, user);
    let off = pack_address(&mut buf, off, mint);
    let off = pack_u64(&mut buf, off, amount);
    pack_u64(&mut buf, off, salt);
    emit_event(&buf);
}

/// Emit `MintCanceled { caller: [u8;32], user: [u8;32], mint: [u8;32], amount: u64, salt: u64 }`
/// Buffer: disc(8) + caller(32) + user(32) + mint(32) + amount(8) + salt(8) = 120 bytes
#[inline]
pub fn emit_mint_canceled(
    caller: &[u8; 32],
    user: &[u8; 32],
    mint: &[u8; 32],
    amount: u64,
    salt: u64,
) {
    pinocchio_log::log!("MintCanceled");
    let mut buf = [0u8; 120];
    let off = pack_disc(&mut buf, &DISC_MINT_CANCELED);
    let off = pack_address(&mut buf, off, caller);
    let off = pack_address(&mut buf, off, user);
    let off = pack_address(&mut buf, off, mint);
    let off = pack_u64(&mut buf, off, amount);
    pack_u64(&mut buf, off, salt);
    emit_event(&buf);
}

/// Emit `DailyLimitUpdated { caller: [u8;32], mint: [u8;32], limit: u64 }`
/// Buffer: disc(8) + caller(32) + mint(32) + limit(8) = 80 bytes
#[inline]
pub fn emit_daily_limit_updated(caller: &[u8; 32], mint: &[u8; 32], limit: u64) {
    pinocchio_log::log!("DailyLimitUpdated");
    let mut buf = [0u8; 80];
    let off = pack_disc(&mut buf, &DISC_DAILY_LIMIT_UPDATED);
    let off = pack_address(&mut buf, off, caller);
    let off = pack_address(&mut buf, off, mint);
    pack_u64(&mut buf, off, limit);
    emit_event(&buf);
}

/// Emit `MaxDelayUpdated { caller: [u8;32], max_delay: i64 }`
/// Buffer: disc(8) + caller(32) + max_delay(8) = 48 bytes
#[inline]
pub fn emit_max_delay_updated(caller: &[u8; 32], max_delay: i64) {
    pinocchio_log::log!("MaxDelayUpdated");
    let mut buf = [0u8; 48];
    let off = pack_disc(&mut buf, &DISC_MAX_DELAY_UPDATED);
    let off = pack_address(&mut buf, off, caller);
    pack_i64(&mut buf, off, max_delay);
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
