//! Structured events for the Spiko Transfer Hook program.

extern crate alloc;

use alloc::vec::Vec;
use spiko_events::{build_event_data, push_address, push_u64};

// SHA256("event:Transfer")[0..8]
const DISC_TRANSFER: [u8; 8] = [0x19, 0x12, 0x17, 0x07, 0xac, 0x74, 0x82, 0x1c];

/// Build `Transfer` event data.
/// Fields: sender (32) + mint (32) + source (32) + destination (32) + amount (8)
#[inline]
pub fn build_transfer_event(
    sender: &[u8; 32],
    mint: &[u8; 32],
    source: &[u8; 32],
    destination: &[u8; 32],
    amount: u64,
) -> Vec<u8> {
    let mut data = build_event_data(&DISC_TRANSFER, 136);
    push_address(&mut data, sender);
    push_address(&mut data, mint);
    push_address(&mut data, source);
    push_address(&mut data, destination);
    push_u64(&mut data, amount);
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
        assert_eq!(DISC_TRANSFER, expected_disc("event:Transfer"));
    }
}
