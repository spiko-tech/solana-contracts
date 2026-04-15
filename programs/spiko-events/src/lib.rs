extern crate alloc;

use alloc::vec::Vec;
use pinocchio::{
    account::AccountView,
    cpi::{invoke_signed, Seed, Signer},
    error::ProgramError,
    instruction::{InstructionAccount, InstructionView},
    Address, ProgramResult,
};

// ─── Constants ───────────────────────────────────────────────────────────────

/// PDA seed used to derive the event authority for each program.
pub const EVENT_AUTHORITY_SEED: &[u8] = b"event_authority";

/// The 1-byte instruction discriminator for the EmitEvent no-op instruction.
/// All programs use this same value.
pub const EMIT_EVENT_DISCRIMINATOR: u8 = 255;

/// Anchor-compatible event CPI instruction tag: SHA256("anchor:event")[0..8].
///
/// This tag is prepended to the CPI instruction data so that Anchor-aware
/// indexers (and the wider Solana tooling ecosystem) can recognise the CPI
/// as an event emission.
/// The raw first 8 bytes of SHA256("anchor:event").
pub const EVENT_IX_TAG_LE: [u8; 8] = [0x1d, 0x9a, 0xcb, 0x51, 0x2e, 0xa5, 0x45, 0xe4];
pub const EVENT_IX_TAG: u64 = u64::from_le_bytes(EVENT_IX_TAG_LE);

// ─── EmitEvent instruction processor (no-op) ─────────────────────────────────

/// Process the `EmitEvent` instruction.
///
/// This is a **no-op** — the only purpose is to validate that the
/// `event_authority` PDA is a signer. The event payload travels as the CPI
/// instruction data and is recorded in the transaction's inner-instruction
/// trace, making it immune to log truncation.
#[inline]
pub fn process_emit_event(
    accounts: &[AccountView],
    expected_event_authority: &Address,
) -> ProgramResult {
    let [event_authority] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    verify_event_authority(event_authority, expected_event_authority)?;

    if !event_authority.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    Ok(())
}

// ─── Verification ────────────────────────────────────────────────────────────

/// Verify that `account` is the expected event authority PDA.
#[inline(always)]
pub fn verify_event_authority(
    account: &AccountView,
    expected: &Address,
) -> Result<(), ProgramError> {
    if account.address() != expected {
        return Err(ProgramError::InvalidSeeds);
    }
    Ok(())
}

// ─── CPI event emission ─────────────────────────────────────────────────────

/// Emit an event via self-CPI to prevent log truncation.
///
/// `event_data` must already contain the full payload:
/// `EVENT_IX_TAG_LE (8) + event_discriminator (8) + LE-packed fields`.
///
/// The function constructs a CPI call to the program's own `EmitEvent`
/// instruction, signed by the `event_authority` PDA.
///
/// The CPI instruction data is: `[255] + event_data`, where 255 is the
/// EmitEvent instruction discriminator used by all programs.
pub fn emit_event(
    program_id: &Address,
    event_authority: &AccountView,
    program: &AccountView,
    event_data: &[u8],
    event_authority_bump: u8,
) -> ProgramResult {
    let bump = [event_authority_bump];
    let signer_seeds: [Seed; 2] = [Seed::from(EVENT_AUTHORITY_SEED), Seed::from(&bump[..])];
    let signer = Signer::from(&signer_seeds);

    let accounts = [InstructionAccount::readonly_signer(
        event_authority.address(),
    )];

    // Prepend the EmitEvent discriminator byte (255) to the CPI data.
    let mut cpi_data = Vec::with_capacity(1 + event_data.len());
    cpi_data.push(EMIT_EVENT_DISCRIMINATOR);
    cpi_data.extend_from_slice(event_data);

    let instruction = InstructionView {
        program_id,
        accounts: &accounts,
        data: &cpi_data,
    };

    invoke_signed(&instruction, &[event_authority, program], &[signer])
}

// ─── Event data builders ─────────────────────────────────────────────────────

/// Build a complete event data buffer: `EVENT_IX_TAG_LE + disc + fields`.
///
/// `capacity` should be `8 (tag) + 8 (disc) + fields_len`.
#[inline]
pub fn build_event_data(disc: &[u8; 8], fields_capacity: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(8 + 8 + fields_capacity);
    data.extend_from_slice(&EVENT_IX_TAG_LE);
    data.extend_from_slice(disc);
    data
}

// ─── Field packing helpers ───────────────────────────────────────────────────

/// Append a 32-byte address to the event data buffer.
#[inline(always)]
pub fn push_address(buf: &mut Vec<u8>, addr: &[u8; 32]) {
    buf.extend_from_slice(addr);
}

/// Append a u64 (LE) to the event data buffer.
#[inline(always)]
pub fn push_u64(buf: &mut Vec<u8>, val: u64) {
    buf.extend_from_slice(&val.to_le_bytes());
}

/// Append an i64 (LE) to the event data buffer.
#[inline(always)]
pub fn push_i64(buf: &mut Vec<u8>, val: i64) {
    buf.extend_from_slice(&val.to_le_bytes());
}

/// Append a u8 to the event data buffer.
#[inline(always)]
pub fn push_u8(buf: &mut Vec<u8>, val: u8) {
    buf.push(val);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_ix_tag_matches_anchor_convention() {
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(b"anchor:event");
        let expected = u64::from_le_bytes(hash[..8].try_into().unwrap());
        assert_eq!(EVENT_IX_TAG, expected);
    }

    #[test]
    fn build_event_data_layout() {
        let disc = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let mut data = build_event_data(&disc, 8);
        push_u64(&mut data, 42);

        assert_eq!(data.len(), 24); // 8 + 8 + 8
        assert_eq!(&data[0..8], &EVENT_IX_TAG_LE);
        assert_eq!(&data[8..16], &disc);
        assert_eq!(&data[16..24], &42u64.to_le_bytes());
    }
}
