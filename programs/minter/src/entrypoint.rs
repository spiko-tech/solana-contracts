use pinocchio::{
    account::AccountView, address::Address, entrypoint, error::ProgramError, ProgramResult,
};

use crate::{
    discriminators::MinterInstructionDiscriminators, events::event_authority_pda, instructions::*,
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

    let ix_discriminator = MinterInstructionDiscriminators::try_from(*discriminator)?;

    match ix_discriminator {
        MinterInstructionDiscriminators::InitializeMinter => {
            InitializeMinter::try_from((data, accounts))?.process(program_id)
        }
        MinterInstructionDiscriminators::InitiateMint => {
            InitiateMint::try_from((data, accounts))?.process(program_id)
        }
        MinterInstructionDiscriminators::ApproveMint => {
            ApproveMint::try_from((data, accounts))?.process(program_id)
        }
        MinterInstructionDiscriminators::CancelMint => {
            CancelMint::try_from((data, accounts))?.process(program_id)
        }
        MinterInstructionDiscriminators::SetDailyLimit => {
            SetDailyLimit::try_from((data, accounts))?.process(program_id)
        }
        MinterInstructionDiscriminators::SetMaxDelay => {
            SetMaxDelay::try_from((data, accounts))?.process(program_id)
        }
        MinterInstructionDiscriminators::EmitEvent => {
            spiko_events::process_emit_event(accounts, &event_authority_pda::ID)
        }
    }
}
