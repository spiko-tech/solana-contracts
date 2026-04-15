use pinocchio::address::Address;

pub mod error;
pub mod events;
pub mod helpers;
pub mod instructions;
pub mod state;

pub use instructions::*;

/// Spiko Token program ID: 8voshdmG84WtPK7Es3okG1tjenfPfviyV66Zmnb1924r
pub const ID: Address = Address::new_from_array([
    0x75, 0xca, 0x97, 0x96, 0x10, 0x70, 0x30, 0x99, 0xeb, 0x06, 0x6d, 0xb0, 0xa2, 0xd9, 0x7f, 0xcb,
    0x48, 0xbc, 0x34, 0x4b, 0x32, 0x01, 0x0f, 0xe8, 0x48, 0x5e, 0xc0, 0x08, 0x07, 0xda, 0x7b, 0xc3,
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
pinocchio::entrypoint!(process_instruction);
fn process_instruction(
    program_id: &Address,
    accounts: &[pinocchio::account::AccountView],
    instruction_data: &[u8],
) -> pinocchio::ProgramResult {
    let (discriminator, data) = instruction_data
        .split_first()
        .ok_or(pinocchio::error::ProgramError::InvalidInstructionData)?;

    match *discriminator {
        0 => InitializeToken::try_from((data, accounts))?.process(program_id),
        1 => MintToken::try_from((data, accounts))?.process(program_id),
        2 => BurnToken::try_from((data, accounts))?.process(program_id),
        3 => TransferToken::try_from((data, accounts))?.process(program_id),
        4 => Pause::try_from((data, accounts))?.process(program_id),
        5 => Unpause::try_from((data, accounts))?.process(program_id),
        6 => RedeemToken::try_from((data, accounts))?.process(program_id),
        7 => SetRedemptionContract::try_from((data, accounts))?.process(program_id),
        8 => BurnFrom::try_from((data, accounts))?.process(program_id),
        EMIT_EVENT_DISCRIMINATOR => {
            spiko_events::process_emit_event(accounts, &event_authority_pda::ID)
        }
        _ => Err(pinocchio::error::ProgramError::InvalidInstructionData),
    }
}
