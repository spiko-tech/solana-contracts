//! Structured events for the Transfer Hook program.
//!
//! Each function emits an Anchor-compatible event via `sol_log_data`:
//! discriminator (8 bytes) = SHA256("event:<EventName>")[0..8], then LE-packed fields.

use spiko_events::{emit_event, pack_address, pack_disc, pack_u64};

// ---------------------------------------------------------------
// Discriminators — precomputed SHA256("event:<EventName>")[0..8]
// ---------------------------------------------------------------

// SHA256("event:Transfer")[0..8]
const DISC_TRANSFER: [u8; 8] = [0x19, 0x12, 0x17, 0x07, 0xac, 0x74, 0x82, 0x1c];

// ---------------------------------------------------------------
// Event emitters
// ---------------------------------------------------------------

/// Emit `Transfer { sender: [u8;32], mint: [u8;32], source: [u8;32], destination: [u8;32], amount: u64 }`
/// Buffer: disc(8) + sender(32) + mint(32) + source(32) + destination(32) + amount(8) = 144 bytes
#[inline]
pub fn emit_transfer(
    sender: &[u8; 32],
    mint: &[u8; 32],
    source: &[u8; 32],
    destination: &[u8; 32],
    amount: u64,
) {
    let mut buf = [0u8; 144];
    let off = pack_disc(&mut buf, &DISC_TRANSFER);
    let off = pack_address(&mut buf, off, sender);
    let off = pack_address(&mut buf, off, mint);
    let off = pack_address(&mut buf, off, source);
    let off = pack_address(&mut buf, off, destination);
    pack_u64(&mut buf, off, amount);
    emit_event(&buf);
}
