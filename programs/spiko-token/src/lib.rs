use pinocchio::address::Address;

pub mod error;
pub mod events;
pub mod helpers;
pub mod instructions;
pub mod state;

pub use instructions::*;

// TODO: Replace with actual deployed program ID bytes
pub const ID: Address = Address::new_from_array([0; 32]);

#[cfg(not(feature = "no-entrypoint"))]
pinocchio::entrypoint!(process_instruction);
fn process_instruction(
    program_id: &Address,
    accounts: &[pinocchio::account::AccountView],
    instruction_data: &[u8],
) -> pinocchio::ProgramResult {
    let (discriminator, data) = instruction_data
        .split_first()
        .ok_or(pinocchio::error::ProgramError::InvalidInstructionData)?;

    match discriminator {
        0 => InitializeToken::try_from((data, accounts))?.process(program_id),
        1 => MintToken::try_from((data, accounts))?.process(program_id),
        2 => BurnToken::try_from((data, accounts))?.process(program_id),
        3 => TransferToken::try_from((data, accounts))?.process(program_id),
        4 => Pause::try_from((data, accounts))?.process(program_id),
        5 => Unpause::try_from((data, accounts))?.process(program_id),
        6 => RedeemToken::try_from((data, accounts))?.process(program_id),
        7 => SetRedemptionContract::try_from((data, accounts))?.process(program_id),
        8 => BurnFrom::try_from((data, accounts))?.process(program_id),
        _ => Err(pinocchio::error::ProgramError::InvalidInstructionData),
    }
}
