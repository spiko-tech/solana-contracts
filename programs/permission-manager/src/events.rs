//! Structured events for the Permission Manager program.
//!
//! Each function builds an Anchor-compatible CPI event payload:
//! `EVENT_IX_TAG (8) + discriminator (8) + LE-packed fields`.
//!
//! Discriminator = SHA256("event:<EventName>")[0..8].

extern crate alloc;

use alloc::vec::Vec;
use spiko_events::{build_event_data, push_address, push_u8};

// SHA256("event:PermissionManagerInitialized")[0..8]
const DISC_PERMISSION_MANAGER_INITIALIZED: [u8; 8] =
    [0xcf, 0x1e, 0x60, 0x38, 0xfd, 0xa9, 0xc5, 0x0f];
// SHA256("event:RoleGranted")[0..8]
const DISC_ROLE_GRANTED: [u8; 8] = [0xdc, 0xb7, 0x59, 0xe4, 0x8f, 0x3f, 0xf6, 0x3a];
// SHA256("event:RoleRemoved")[0..8]
const DISC_ROLE_REMOVED: [u8; 8] = [0x85, 0x23, 0xd6, 0xea, 0xcb, 0x9d, 0xcb, 0x35];
// SHA256("event:OwnershipTransferStarted")[0..8]
const DISC_OWNERSHIP_TRANSFER_STARTED: [u8; 8] = [0xb7, 0xfd, 0xef, 0xf6, 0x8c, 0xb3, 0x85, 0x69];
// SHA256("event:OwnershipTransferred")[0..8]
const DISC_OWNERSHIP_TRANSFERRED: [u8; 8] = [0xac, 0x3d, 0xcd, 0xb7, 0xfa, 0x32, 0x26, 0x62];

/// Build `PermissionManagerInitialized` event data.
/// Fields: admin (32 bytes)
#[inline]
pub fn build_permission_manager_initialized_event(admin: &[u8; 32]) -> Vec<u8> {
    let mut data = build_event_data(&DISC_PERMISSION_MANAGER_INITIALIZED, 32);
    push_address(&mut data, admin);
    data
}

/// Build `RoleGranted` event data.
/// Fields: caller (32) + target (32) + role_id (1)
#[inline]
pub fn build_role_granted_event(caller: &[u8; 32], target: &[u8; 32], role_id: u8) -> Vec<u8> {
    let mut data = build_event_data(&DISC_ROLE_GRANTED, 65);
    push_address(&mut data, caller);
    push_address(&mut data, target);
    push_u8(&mut data, role_id);
    data
}

/// Build `RoleRemoved` event data.
/// Fields: caller (32) + target (32) + role_id (1)
#[inline]
pub fn build_role_removed_event(caller: &[u8; 32], target: &[u8; 32], role_id: u8) -> Vec<u8> {
    let mut data = build_event_data(&DISC_ROLE_REMOVED, 65);
    push_address(&mut data, caller);
    push_address(&mut data, target);
    push_u8(&mut data, role_id);
    data
}

/// Build `OwnershipTransferStarted` event data.
/// Fields: admin (32) + new_admin (32)
#[inline]
pub fn build_ownership_transfer_started_event(admin: &[u8; 32], new_admin: &[u8; 32]) -> Vec<u8> {
    let mut data = build_event_data(&DISC_OWNERSHIP_TRANSFER_STARTED, 64);
    push_address(&mut data, admin);
    push_address(&mut data, new_admin);
    data
}

/// Build `OwnershipTransferred` event data.
/// Fields: new_admin (32)
#[inline]
pub fn build_ownership_transferred_event(new_admin: &[u8; 32]) -> Vec<u8> {
    let mut data = build_event_data(&DISC_OWNERSHIP_TRANSFERRED, 32);
    push_address(&mut data, new_admin);
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
            DISC_PERMISSION_MANAGER_INITIALIZED,
            expected_disc("event:PermissionManagerInitialized")
        );
        assert_eq!(DISC_ROLE_GRANTED, expected_disc("event:RoleGranted"));
        assert_eq!(DISC_ROLE_REMOVED, expected_disc("event:RoleRemoved"));
        assert_eq!(
            DISC_OWNERSHIP_TRANSFER_STARTED,
            expected_disc("event:OwnershipTransferStarted")
        );
        assert_eq!(
            DISC_OWNERSHIP_TRANSFERRED,
            expected_disc("event:OwnershipTransferred")
        );
    }
}
