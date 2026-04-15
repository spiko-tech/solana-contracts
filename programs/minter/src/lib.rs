use pinocchio::{
    account::AccountView, address::Address, entrypoint, error::ProgramError, ProgramResult,
};

pub mod error;
pub mod events;
pub mod helpers;
pub mod instructions;
pub mod state;

pub use instructions::*;

/// Minter program ID: 3Ex4bd3DeBtV6k15z1heA9TZb3otbEFErznfkC1Vdwhn
pub const ID: Address = Address::new_from_array([
    0x21, 0x4a, 0x2e, 0xe5, 0x92, 0x86, 0x0e, 0x74, 0x40, 0xb1, 0x70, 0x5a, 0x01, 0xbb, 0xdf, 0x8b,
    0xba, 0x5e, 0x51, 0x60, 0x9a, 0x00, 0x07, 0x5f, 0xa1, 0x2d, 0xa3, 0xbc, 0x83, 0x04, 0x86, 0x75,
]);

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
