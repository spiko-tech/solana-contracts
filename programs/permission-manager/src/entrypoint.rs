use pinocchio::{
    account::AccountView, address::Address, entrypoint, error::ProgramError, ProgramResult,
};

use crate::{
    discriminators::PermissionManagerInstructionDiscriminators, events::event_authority_pda,
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

    let ix_discriminator = PermissionManagerInstructionDiscriminators::try_from(*discriminator)?;

    match ix_discriminator {
        PermissionManagerInstructionDiscriminators::Initialize => {
            Initialize::try_from((data, accounts))?.process(program_id)
        }
        PermissionManagerInstructionDiscriminators::GrantRole => {
            GrantRole::try_from((data, accounts))?.process(program_id)
        }
        PermissionManagerInstructionDiscriminators::RevokeRole => {
            RevokeRole::try_from((data, accounts))?.process(program_id)
        }
        PermissionManagerInstructionDiscriminators::TransferOwnership => {
            TransferOwnership::try_from((data, accounts))?.process(program_id)
        }
        PermissionManagerInstructionDiscriminators::AcceptOwnership => {
            AcceptOwnership::try_from((data, accounts))?.process(program_id)
        }
        PermissionManagerInstructionDiscriminators::EmitEvent => {
            spiko_events::process_emit_event(accounts, &event_authority_pda::ID)
        }
    }
}
