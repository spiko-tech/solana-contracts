use pinocchio::{
    account::AccountView, address::Address, entrypoint, error::ProgramError, ProgramResult,
};

use crate::{
    discriminators::GatekeeperInstructionDiscriminators, events::event_authority_pda,
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

    let ix_discriminator = GatekeeperInstructionDiscriminators::try_from(*discriminator)?;

    match ix_discriminator {
        GatekeeperInstructionDiscriminators::Initialize => {
            Initialize::try_from((data, accounts))?.process(program_id)
        }
        GatekeeperInstructionDiscriminators::SetDailyLimit => {
            SetDailyLimit::try_from((data, accounts))?.process(program_id)
        }
        GatekeeperInstructionDiscriminators::CustodialWithdraw => {
            CustodialWithdraw::try_from((data, accounts))?.process(program_id)
        }
        GatekeeperInstructionDiscriminators::ApproveWithdrawal => {
            ApproveWithdrawal::try_from((data, accounts))?.process(program_id)
        }
        GatekeeperInstructionDiscriminators::CancelWithdrawal => {
            CancelWithdrawal::try_from((data, accounts))?.process(program_id)
        }
        GatekeeperInstructionDiscriminators::EmitEvent => {
            spiko_events::process_emit_event(accounts, &event_authority_pda::ID)
        }
    }
}
