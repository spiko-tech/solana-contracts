//! Codama event type definitions for the Permission Manager program.
//!
//! These structs describe the event schema for IDL generation.
//! They do NOT replace the existing event builder functions in `events.rs`.

use codama::CodamaType;
use pinocchio::Address;

/// Emitted when the permission manager is initialized.
#[derive(CodamaType)]
pub struct PermissionManagerInitializedEvent {
    pub admin: Address,
}

/// Emitted when a role is granted to a user.
#[derive(CodamaType)]
pub struct RoleGrantedEvent {
    pub caller: Address,
    pub target: Address,
    pub role_id: u8,
}

/// Emitted when a role is revoked from a user.
#[derive(CodamaType)]
pub struct RoleRemovedEvent {
    pub caller: Address,
    pub target: Address,
    pub role_id: u8,
}

/// Emitted when an ownership transfer is initiated.
#[derive(CodamaType)]
pub struct OwnershipTransferStartedEvent {
    pub admin: Address,
    pub new_admin: Address,
}

/// Emitted when an ownership transfer is completed.
#[derive(CodamaType)]
pub struct OwnershipTransferredEvent {
    pub new_admin: Address,
}
