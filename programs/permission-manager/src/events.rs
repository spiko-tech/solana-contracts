//! Structured events for the Permission Manager program.
//!
//! Each function emits an Anchor-compatible event via `sol_log_data`:
//! discriminator (8 bytes) = SHA256("event:<EventName>")[0..8], then LE-packed fields.

use spiko_events::{emit_event, pack_address, pack_disc, pack_u8};

// ---------------------------------------------------------------
// Discriminators — precomputed SHA256("event:<EventName>")[0..8]
// ---------------------------------------------------------------

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

// ---------------------------------------------------------------
// Event emitters
// ---------------------------------------------------------------

/// Emit `PermissionManagerInitialized { admin: [u8;32] }`
/// Buffer: disc(8) + admin(32) = 40 bytes
#[inline]
pub fn emit_permission_manager_initialized(admin: &[u8; 32]) {
    let mut buf = [0u8; 40];
    let off = pack_disc(&mut buf, &DISC_PERMISSION_MANAGER_INITIALIZED);
    pack_address(&mut buf, off, admin);
    emit_event(&buf);
}

/// Emit `RoleGranted { caller: [u8;32], target: [u8;32], role_id: u8 }`
/// Buffer: disc(8) + caller(32) + target(32) + role_id(1) = 73 bytes
#[inline]
pub fn emit_role_granted(caller: &[u8; 32], target: &[u8; 32], role_id: u8) {
    let mut buf = [0u8; 73];
    let off = pack_disc(&mut buf, &DISC_ROLE_GRANTED);
    let off = pack_address(&mut buf, off, caller);
    let off = pack_address(&mut buf, off, target);
    pack_u8(&mut buf, off, role_id);
    emit_event(&buf);
}

/// Emit `RoleRemoved { caller: [u8;32], target: [u8;32], role_id: u8 }`
/// Buffer: disc(8) + caller(32) + target(32) + role_id(1) = 73 bytes
#[inline]
pub fn emit_role_removed(caller: &[u8; 32], target: &[u8; 32], role_id: u8) {
    let mut buf = [0u8; 73];
    let off = pack_disc(&mut buf, &DISC_ROLE_REMOVED);
    let off = pack_address(&mut buf, off, caller);
    let off = pack_address(&mut buf, off, target);
    pack_u8(&mut buf, off, role_id);
    emit_event(&buf);
}

/// Emit `OwnershipTransferStarted { admin: [u8;32], new_admin: [u8;32] }`
/// Buffer: disc(8) + admin(32) + new_admin(32) = 72 bytes
#[inline]
pub fn emit_ownership_transfer_started(admin: &[u8; 32], new_admin: &[u8; 32]) {
    let mut buf = [0u8; 72];
    let off = pack_disc(&mut buf, &DISC_OWNERSHIP_TRANSFER_STARTED);
    let off = pack_address(&mut buf, off, admin);
    pack_address(&mut buf, off, new_admin);
    emit_event(&buf);
}

/// Emit `OwnershipTransferred { new_admin: [u8;32] }`
/// Buffer: disc(8) + new_admin(32) = 40 bytes
#[inline]
pub fn emit_ownership_transferred(new_admin: &[u8; 32]) {
    let mut buf = [0u8; 40];
    let off = pack_disc(&mut buf, &DISC_OWNERSHIP_TRANSFERRED);
    pack_address(&mut buf, off, new_admin);
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
