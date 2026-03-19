//! Structured events for the SpikoToken program.
//!
//! Each function emits an Anchor-compatible event via `sol_log_data`:
//! discriminator (8 bytes) = SHA256("event:<EventName>")[0..8], then LE-packed fields.

use spiko_events::{emit_event, pack_address, pack_disc, pack_u64};

// ---------------------------------------------------------------
// Discriminators — precomputed SHA256("event:<EventName>")[0..8]
// ---------------------------------------------------------------

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
// SHA256("event:DelegateApproved")[0..8]
const DISC_DELEGATE_APPROVED: [u8; 8] = [0xb4, 0xda, 0x13, 0x17, 0x68, 0xb0, 0x70, 0x7b];

// ---------------------------------------------------------------
// Event emitters
// ---------------------------------------------------------------

/// Emit `TokenInitialized { admin: [u8;32], mint: [u8;32] }`
/// Buffer: disc(8) + admin(32) + mint(32) = 72 bytes
#[inline]
pub fn emit_token_initialized(admin: &[u8; 32], mint: &[u8; 32]) {
    let mut buf = [0u8; 72];
    let off = pack_disc(&mut buf, &DISC_TOKEN_INITIALIZED);
    let off = pack_address(&mut buf, off, admin);
    pack_address(&mut buf, off, mint);
    emit_event(&buf);
}

/// Emit `Mint { caller: [u8;32], mint: [u8;32], recipient_ata: [u8;32], amount: u64 }`
/// Buffer: disc(8) + caller(32) + mint(32) + recipient_ata(32) + amount(8) = 112 bytes
#[inline]
pub fn emit_mint(caller: &[u8; 32], mint: &[u8; 32], recipient_ata: &[u8; 32], amount: u64) {
    let mut buf = [0u8; 112];
    let off = pack_disc(&mut buf, &DISC_MINT);
    let off = pack_address(&mut buf, off, caller);
    let off = pack_address(&mut buf, off, mint);
    let off = pack_address(&mut buf, off, recipient_ata);
    pack_u64(&mut buf, off, amount);
    emit_event(&buf);
}

/// Emit `Burn { caller: [u8;32], mint: [u8;32], source_ata: [u8;32], amount: u64 }`
/// Buffer: disc(8) + caller(32) + mint(32) + source_ata(32) + amount(8) = 112 bytes
#[inline]
pub fn emit_burn(caller: &[u8; 32], mint: &[u8; 32], source_ata: &[u8; 32], amount: u64) {
    let mut buf = [0u8; 112];
    let off = pack_disc(&mut buf, &DISC_BURN);
    let off = pack_address(&mut buf, off, caller);
    let off = pack_address(&mut buf, off, mint);
    let off = pack_address(&mut buf, off, source_ata);
    pack_u64(&mut buf, off, amount);
    emit_event(&buf);
}

/// Emit `RedeemInitiated { user: [u8;32], mint: [u8;32], amount: u64, salt: u64 }`
/// Buffer: disc(8) + user(32) + mint(32) + amount(8) + salt(8) = 88 bytes
#[inline]
pub fn emit_redeem_initiated(user: &[u8; 32], mint: &[u8; 32], amount: u64, salt: u64) {
    let mut buf = [0u8; 88];
    let off = pack_disc(&mut buf, &DISC_REDEEM_INITIATED);
    let off = pack_address(&mut buf, off, user);
    let off = pack_address(&mut buf, off, mint);
    let off = pack_u64(&mut buf, off, amount);
    pack_u64(&mut buf, off, salt);
    emit_event(&buf);
}

/// Emit `TokenPaused { caller: [u8;32], config: [u8;32] }`
/// Buffer: disc(8) + caller(32) + config(32) = 72 bytes
#[inline]
pub fn emit_token_paused(caller: &[u8; 32], config: &[u8; 32]) {
    let mut buf = [0u8; 72];
    let off = pack_disc(&mut buf, &DISC_TOKEN_PAUSED);
    let off = pack_address(&mut buf, off, caller);
    pack_address(&mut buf, off, config);
    emit_event(&buf);
}

/// Emit `TokenUnpaused { caller: [u8;32], config: [u8;32] }`
/// Buffer: disc(8) + caller(32) + config(32) = 72 bytes
#[inline]
pub fn emit_token_unpaused(caller: &[u8; 32], config: &[u8; 32]) {
    let mut buf = [0u8; 72];
    let off = pack_disc(&mut buf, &DISC_TOKEN_UNPAUSED);
    let off = pack_address(&mut buf, off, caller);
    pack_address(&mut buf, off, config);
    emit_event(&buf);
}

/// Emit `RedemptionContractSet { caller: [u8;32], config: [u8;32], contract: [u8;32] }`
/// Buffer: disc(8) + caller(32) + config(32) + contract(32) = 104 bytes
#[inline]
pub fn emit_redemption_contract_set(caller: &[u8; 32], config: &[u8; 32], contract: &[u8; 32]) {
    let mut buf = [0u8; 104];
    let off = pack_disc(&mut buf, &DISC_REDEMPTION_CONTRACT_SET);
    let off = pack_address(&mut buf, off, caller);
    let off = pack_address(&mut buf, off, config);
    pack_address(&mut buf, off, contract);
    emit_event(&buf);
}

/// Emit `DelegateApproved { owner: [u8;32], token_account: [u8;32], delegate: [u8;32], amount: u64 }`
/// Buffer: disc(8) + owner(32) + token_account(32) + delegate(32) + amount(8) = 112 bytes
#[inline]
pub fn emit_delegate_approved(
    owner: &[u8; 32],
    token_account: &[u8; 32],
    delegate: &[u8; 32],
    amount: u64,
) {
    let mut buf = [0u8; 112];
    let off = pack_disc(&mut buf, &DISC_DELEGATE_APPROVED);
    let off = pack_address(&mut buf, off, owner);
    let off = pack_address(&mut buf, off, token_account);
    let off = pack_address(&mut buf, off, delegate);
    pack_u64(&mut buf, off, amount);
    emit_event(&buf);
}
