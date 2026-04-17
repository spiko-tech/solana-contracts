use pinocchio::{
    account::AccountView, address::Address, entrypoint, error::ProgramError, ProgramResult,
};

use crate::{
    discriminators::RedemptionInstructionDiscriminators, events::event_authority_pda,
    instructions::*,
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

    let ix_discriminator = RedemptionInstructionDiscriminators::try_from(*discriminator)?;

    match ix_discriminator {
        RedemptionInstructionDiscriminators::InitializeRedemption => {
            InitializeRedemption::try_from((data, accounts))?.process(program_id)
        }
        RedemptionInstructionDiscriminators::ExecuteRedemption => {
            ExecuteRedemption::try_from((data, accounts))?.process(program_id)
        }
        RedemptionInstructionDiscriminators::CancelRedemption => {
            CancelRedemption::try_from((data, accounts))?.process(program_id)
        }
        RedemptionInstructionDiscriminators::SetMinimum => {
            SetMinimum::try_from((data, accounts))?.process(program_id)
        }
        RedemptionInstructionDiscriminators::OnRedeem => {
            OnRedeem::try_from((data, accounts))?.process(program_id)
        }
        RedemptionInstructionDiscriminators::EmitEvent => {
            spiko_events::process_emit_event(accounts, &event_authority_pda::ID)
        }
    }
}
