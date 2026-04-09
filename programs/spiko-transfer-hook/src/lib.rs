use pinocchio::address::Address;

pub mod error;
pub mod events;
pub mod helpers;
pub mod instructions;
pub mod state;

pub use instructions::*;

pub const ID: Address = Address::new_from_array([0; 32]);

/// Transfer Hook Execute sighash: SHA256("spl-transfer-hook-interface:execute")[0..8]
#[cfg(not(feature = "no-entrypoint"))]
const EXECUTE_SIGHASH: [u8; 8] = [105, 37, 101, 197, 75, 251, 102, 26];

#[cfg(not(feature = "no-entrypoint"))]
pinocchio::entrypoint!(process_instruction);

#[cfg(not(feature = "no-entrypoint"))]
fn process_instruction(
    program_id: &Address,
    accounts: &[pinocchio::account::AccountView],
    instruction_data: &[u8],
) -> pinocchio::ProgramResult {
    // First, check for the 8-byte Transfer Hook Execute sighash.
    // Token-2022 sends this discriminator when invoking the hook.
    if instruction_data.len() >= 8 && instruction_data[0..8] == EXECUTE_SIGHASH {
        // Data after the 8-byte sighash: amount (u64 LE)
        return TransferHookExecute::try_from((&instruction_data[8..], accounts))?
            .process(program_id);
    }

    // Otherwise, use the standard 1-byte discriminator dispatch.
    let (discriminator, data) = instruction_data
        .split_first()
        .ok_or(pinocchio::error::ProgramError::InvalidInstructionData)?;

    match discriminator {
        0 => InitExtraAccountMetas::try_from((data, accounts))?.process(program_id),
        _ => Err(pinocchio::error::ProgramError::InvalidInstructionData),
    }
}
