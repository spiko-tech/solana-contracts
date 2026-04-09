use pinocchio::{
    account::AccountView, address::Address, entrypoint, error::ProgramError, ProgramResult,
};

pub mod error;
pub mod events;
pub mod helpers;
pub mod instructions;
pub mod state;

pub use instructions::*;

// TODO: Replace with actual deployed program ID bytes
pub const ID: Address = Address::new_from_array([0; 32]);

#[cfg(not(feature = "no-entrypoint"))]
entrypoint!(process_instruction);

fn process_instruction(
    program_id: &Address,
    accounts: &[AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    let (discriminator, data) = instruction_data
        .split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;

    match discriminator {
        0 => InitializeRedemption::try_from((data, accounts))?.process(program_id),
        1 => ExecuteRedemption::try_from((data, accounts))?.process(program_id),
        2 => CancelRedemption::try_from((data, accounts))?.process(program_id),
        3 => SetMinimum::try_from((data, accounts))?.process(program_id),
        4 => OnRedeem::try_from((data, accounts))?.process(program_id),
        _ => Err(ProgramError::InvalidInstructionData),
    }
}
