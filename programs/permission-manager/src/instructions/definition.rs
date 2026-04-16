//! Codama instruction definitions for the Permission Manager program.
//!
//! This file is used exclusively for IDL generation via the `codama` crate.
//! It does NOT replace the actual dispatch logic in `lib.rs`.

use codama::CodamaInstructions;
use pinocchio::Address;

/// Instructions for the Permission Manager program.
#[allow(clippy::large_enum_variant)]
#[repr(C, u8)]
#[derive(Clone, Debug, PartialEq, CodamaInstructions)]
pub enum PermissionManagerInstruction {
    /// Initialize the permission manager with an admin.
    #[codama(account(
        name = "admin",
        docs = "Admin / deployer (payer, becomes initial admin)",
        signer,
        writable
    ))]
    #[codama(account(
        name = "config",
        docs = "PermissionConfig PDA to be created",
        writable,
        default_value = pda("permissionConfig", [])
    ))]
    #[codama(account(
        name = "admin_perms",
        docs = "Admin's UserPermissions PDA to be created",
        writable,
        default_value = pda("userPermissions", [seed("user", account("admin"))])
    ))]
    #[codama(account(name = "system_program", docs = "System program", default_value = program("system")))]
    #[codama(account(
        name = "event_authority",
        docs = "Event authority PDA for CPI event emission",
        default_value = public_key("H6L8Npy9RazKzX8RFetnM2obso49gB93qocxySvJcNLs")
    ))]
    #[codama(account(
        name = "self_program",
        docs = "Permission Manager program (self) for CPI event emission",
        default_value = public_key("2Qhjh6NXiyQEPBP9tVCkzNtLWERHbggUjbbwje1Mpqsc")
    ))]
    Initialize {} = 0,

    /// Grant a role to a target user.
    #[codama(account(
        name = "caller",
        docs = "Caller (payer if target PDA needs creation)",
        signer,
        writable
    ))]
    #[codama(account(name = "config", docs = "PermissionConfig PDA"))]
    #[codama(account(
        name = "user_perms",
        docs = "Target user's UserPermissions PDA (created if needed)",
        writable,
        default_value = pda("userPermissions", [seed("user", account("target_user"))])
    ))]
    #[codama(account(name = "system_program", docs = "System program", default_value = program("system")))]
    #[codama(account(
        name = "target_user",
        docs = "Target user address (for PDA derivation)"
    ))]
    #[codama(account(
        name = "caller_perms",
        docs = "Caller's UserPermissions PDA (for role-hierarchy check)",
        default_value = pda("userPermissions", [seed("user", account("caller"))])
    ))]
    #[codama(account(
        name = "event_authority",
        docs = "Event authority PDA for CPI event emission",
        default_value = public_key("H6L8Npy9RazKzX8RFetnM2obso49gB93qocxySvJcNLs")
    ))]
    #[codama(account(
        name = "self_program",
        docs = "Permission Manager program (self) for CPI event emission",
        default_value = public_key("2Qhjh6NXiyQEPBP9tVCkzNtLWERHbggUjbbwje1Mpqsc")
    ))]
    GrantRole {
        /// Role identifier to grant
        role_id: u8,
    } = 1,

    /// Revoke a role from a target user.
    #[codama(account(name = "caller", docs = "Caller (admin or role manager)", signer))]
    #[codama(account(name = "config", docs = "PermissionConfig PDA"))]
    #[codama(account(
        name = "user_perms",
        docs = "Target user's UserPermissions PDA",
        writable,
        default_value = pda("userPermissions", [seed("user", account("target_user"))])
    ))]
    #[codama(account(
        name = "target_user",
        docs = "Target user address (for PDA derivation)"
    ))]
    #[codama(account(
        name = "caller_perms",
        docs = "Caller's UserPermissions PDA (for role-hierarchy check)",
        default_value = pda("userPermissions", [seed("user", account("caller"))])
    ))]
    #[codama(account(
        name = "event_authority",
        docs = "Event authority PDA for CPI event emission",
        default_value = public_key("H6L8Npy9RazKzX8RFetnM2obso49gB93qocxySvJcNLs")
    ))]
    #[codama(account(
        name = "self_program",
        docs = "Permission Manager program (self) for CPI event emission",
        default_value = public_key("2Qhjh6NXiyQEPBP9tVCkzNtLWERHbggUjbbwje1Mpqsc")
    ))]
    RevokeRole {
        /// Role identifier to revoke
        role_id: u8,
    } = 2,

    /// Start ownership transfer to a new admin.
    #[codama(account(name = "admin", docs = "Current admin", signer))]
    #[codama(account(
        name = "config",
        docs = "PermissionConfig PDA",
        writable,
        default_value = pda("permissionConfig", [])
    ))]
    #[codama(account(
        name = "event_authority",
        docs = "Event authority PDA for CPI event emission",
        default_value = public_key("H6L8Npy9RazKzX8RFetnM2obso49gB93qocxySvJcNLs")
    ))]
    #[codama(account(
        name = "self_program",
        docs = "Permission Manager program (self) for CPI event emission",
        default_value = public_key("2Qhjh6NXiyQEPBP9tVCkzNtLWERHbggUjbbwje1Mpqsc")
    ))]
    TransferOwnership {
        /// New admin address to transfer ownership to
        new_admin: Address,
    } = 3,

    /// Accept ownership transfer as the pending admin.
    #[codama(account(
        name = "new_admin",
        docs = "New admin (must be the pending_admin)",
        signer
    ))]
    #[codama(account(
        name = "config",
        docs = "PermissionConfig PDA",
        writable,
        default_value = pda("permissionConfig", [])
    ))]
    #[codama(account(
        name = "event_authority",
        docs = "Event authority PDA for CPI event emission",
        default_value = public_key("H6L8Npy9RazKzX8RFetnM2obso49gB93qocxySvJcNLs")
    ))]
    #[codama(account(
        name = "self_program",
        docs = "Permission Manager program (self) for CPI event emission",
        default_value = public_key("2Qhjh6NXiyQEPBP9tVCkzNtLWERHbggUjbbwje1Mpqsc")
    ))]
    AcceptOwnership {} = 4,

    /// Invoked via CPI to emit event data in instruction args.
    #[codama(skip)]
    EmitEvent {} = 255,
}
