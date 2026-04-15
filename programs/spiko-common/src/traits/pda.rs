use pinocchio::{account::AccountView, error::ProgramError, Address};

/// PDA seed generation tied to state structs.
///
/// Implementors define `PREFIX` to identify their PDA type, and implement
/// `validate_pda_address()` to verify an account matches the expected PDA.
pub trait PdaSeeds {
    /// Static prefix seed (e.g., b"token_config").
    const PREFIX: &'static [u8];

    /// Validate that an account matches the canonical PDA for this state,
    /// and return the bump.
    ///
    /// Uses `find_program_address` (~1500 CU). When the bump is already known
    /// (e.g., after deserialization), prefer `PdaAccount::validate_self()`.
    fn validate_pda_address(
        &self,
        account: &AccountView,
        program_id: &Address,
    ) -> Result<u8, ProgramError>;
}

/// Extension trait for account types that store their PDA bump.
///
/// Provides `validate_self()` which uses `Address::derive_address` with the
/// known bump -- cheaper than `find_program_address` (~200 CU vs ~1500 CU).
pub trait PdaAccount: PdaSeeds {
    /// Returns the stored bump seed for this account's PDA.
    fn bump(&self) -> u8;

    /// Validate that the account matches the PDA derived from this state's
    /// seeds and stored bump. Uses the cheaper `derive_address`.
    fn validate_self(
        &self,
        account: &AccountView,
        program_id: &Address,
    ) -> Result<(), ProgramError>;
}
