use pinocchio::address::{declare_id, Address};

pub mod error;
pub mod event_types;
pub mod events;
pub mod helpers;
pub mod instructions;
pub mod state;

pub use instructions::*;

declare_id!("CKV53PkgjvoTmfpzdkbuQc9fMukqu7Qey7kLoSiTwYmY");

/// Event authority PDA derived at compile time from the program ID.
pub mod event_authority_pda {
    use const_crypto::ed25519;
    use pinocchio::address::Address;
    use spiko_events::EVENT_AUTHORITY_SEED;

    const RESULT: ([u8; 32], u8) =
        ed25519::derive_program_address(&[EVENT_AUTHORITY_SEED], crate::ID.as_array());

    pub const ID: Address = Address::new_from_array(RESULT.0);
    pub const BUMP: u8 = RESULT.1;
}

/// Discriminator for the EmitEvent instruction.
pub const EMIT_EVENT_DISCRIMINATOR: u8 = 255;

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

    match *discriminator {
        0 => InitExtraAccountMetas::try_from((data, accounts))?.process(program_id),
        EMIT_EVENT_DISCRIMINATOR => {
            spiko_events::process_emit_event(accounts, &event_authority_pda::ID)
        }
        _ => Err(pinocchio::error::ProgramError::InvalidInstructionData),
    }
}
