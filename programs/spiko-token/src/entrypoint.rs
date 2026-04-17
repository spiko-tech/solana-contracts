use pinocchio::{
    account::AccountView, address::Address, entrypoint, error::ProgramError, ProgramResult,
};

use crate::{
    discriminators::TokenInstructionDiscriminators, events::event_authority_pda, instructions::*,
};

entrypoint!(process_instruction);

fn process_instruction(
    program_id: &Address,
    accounts: &[AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    let (discriminator, data) = instruction_data
        .split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;

    let ix_discriminator = TokenInstructionDiscriminators::try_from(*discriminator)?;

    match ix_discriminator {
        TokenInstructionDiscriminators::InitializeToken => {
            InitializeToken::try_from((data, accounts))?.process(program_id)
        }
        TokenInstructionDiscriminators::MintToken => {
            MintToken::try_from((data, accounts))?.process(program_id)
        }
        TokenInstructionDiscriminators::BurnToken => {
            BurnToken::try_from((data, accounts))?.process(program_id)
        }
        TokenInstructionDiscriminators::TransferToken => {
            TransferToken::try_from((data, accounts))?.process(program_id)
        }
        TokenInstructionDiscriminators::Pause => {
            Pause::try_from((data, accounts))?.process(program_id)
        }
        TokenInstructionDiscriminators::Unpause => {
            Unpause::try_from((data, accounts))?.process(program_id)
        }
        TokenInstructionDiscriminators::RedeemToken => {
            RedeemToken::try_from((data, accounts))?.process(program_id)
        }
        TokenInstructionDiscriminators::SetRedemptionContract => {
            SetRedemptionContract::try_from((data, accounts))?.process(program_id)
        }
        TokenInstructionDiscriminators::BurnFrom => {
            BurnFrom::try_from((data, accounts))?.process(program_id)
        }
        TokenInstructionDiscriminators::EmitEvent => {
            spiko_events::process_emit_event(accounts, &event_authority_pda::ID)
        }
    }
}
