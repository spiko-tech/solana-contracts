use pinocchio::{
    account::AccountView,
    address::{declare_id, Address},
    entrypoint,
    error::ProgramError,
    ProgramResult,
};

pub mod error;
pub mod event_types;
pub mod events;
pub mod helpers;
pub mod instructions;
pub mod state;

pub use instructions::*;

declare_id!("3pXknoeMQiY44nKBcnwtSSxzuh1uxUHPHggjXcuVLDT2");

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

    match *discriminator {
        0 => InitializeMinter::try_from((data, accounts))?.process(program_id),
        1 => InitiateMint::try_from((data, accounts))?.process(program_id),
        2 => ApproveMint::try_from((data, accounts))?.process(program_id),
        3 => CancelMint::try_from((data, accounts))?.process(program_id),
        4 => SetDailyLimit::try_from((data, accounts))?.process(program_id),
        5 => SetMaxDelay::try_from((data, accounts))?.process(program_id),
        EMIT_EVENT_DISCRIMINATOR => {
            spiko_events::process_emit_event(accounts, &event_authority_pda::ID)
        }
        _ => Err(ProgramError::InvalidInstructionData),
    }
}
