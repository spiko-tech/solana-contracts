use pinocchio::{
    account::AccountView, address::Address, entrypoint, error::ProgramError, ProgramResult,
};

pub mod error;
pub mod events;
pub mod helpers;
pub mod instructions;
pub mod state;

pub use instructions::*;

/// Redemption program ID: 7rwaFEEkYYHWx3U5UfidGVp5JyiB5VkPcLAxRYtkBHxi
pub const ID: Address = Address::new_from_array([
    0x65, 0xf1, 0x49, 0x8d, 0x39, 0xc4, 0xff, 0xdb, 0x73, 0x89, 0x2a, 0x4c, 0xc9, 0x9f, 0x93, 0xee,
    0xf6, 0x81, 0x63, 0x37, 0x11, 0xae, 0xea, 0x3f, 0xa7, 0x3c, 0x4a, 0xea, 0xe0, 0x5c, 0xc7, 0xbf,
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
        0 => InitializeRedemption::try_from((data, accounts))?.process(program_id),
        1 => ExecuteRedemption::try_from((data, accounts))?.process(program_id),
        2 => CancelRedemption::try_from((data, accounts))?.process(program_id),
        3 => SetMinimum::try_from((data, accounts))?.process(program_id),
        4 => OnRedeem::try_from((data, accounts))?.process(program_id),
        EMIT_EVENT_DISCRIMINATOR => {
            spiko_events::process_emit_event(accounts, &event_authority_pda::ID)
        }
        _ => Err(ProgramError::InvalidInstructionData),
    }
}
