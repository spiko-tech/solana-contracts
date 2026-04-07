use pinocchio::{
    account::AccountView, address::Address, cpi::Signer, error::ProgramError, ProgramResult,
};

/// Verify that an account's address matches the expected PDA derived from seeds,
/// and return the bump. Uses `find_program_address` (~1500 CU).
#[inline]
pub fn verify_pda(
    account: &AccountView,
    seeds: &[&[u8]],
    program_id: &Address,
) -> Result<u8, ProgramError> {
    let (expected, bump) = Address::find_program_address(seeds, program_id);
    if account.address() != &expected {
        return Err(ProgramError::InvalidSeeds);
    }
    Ok(bump)
}

/// Create a PDA account via CPI to the system program. The PDA signs.
pub fn create_pda_account(
    payer: &AccountView,
    pda_account: &AccountView,
    space: usize,
    owner: &Address,
    signer_seeds: &[Signer],
) -> ProgramResult {
    pinocchio_system::create_account_with_minimum_balance_signed(
        pda_account,
        space,
        owner,
        payer,
        None, // uses Rent::get() syscall internally
        signer_seeds,
    )
}
