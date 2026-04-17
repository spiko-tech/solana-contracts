use const_crypto::ed25519;
use pinocchio::address::Address;
use spiko_events::EVENT_AUTHORITY_SEED;

/// Event Authority PDA derived at compile time from the program ID.
pub mod event_authority_pda {
    use super::*;

    const RESULT: ([u8; 32], u8) =
        ed25519::derive_program_address(&[EVENT_AUTHORITY_SEED], crate::ID.as_array());

    pub const ID: Address = Address::new_from_array(RESULT.0);
    pub const BUMP: u8 = RESULT.1;
}
