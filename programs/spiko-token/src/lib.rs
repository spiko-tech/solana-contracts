use pinocchio::address::{declare_id, Address};

pub mod error;
pub mod event_types;
pub mod events;
pub mod helpers;
pub mod instructions;
pub mod state;

pub use instructions::*;

declare_id!("3V5sE4AFgkS8T8Jrt41wK8t2rJXo9VhURt6AGfqar9Zd");

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
