use pinocchio::{
    account::AccountView, address::Address, entrypoint, error::ProgramError, ProgramResult,
};

use crate::{
    discriminators::TransferHookInstructionDiscriminators, events::event_authority_pda,
    instructions::*,
};

/// Transfer Hook Execute sighash: SHA256("spl-transfer-hook-interface:execute")[0..8]
const EXECUTE_SIGHASH: [u8; 8] = [105, 37, 101, 197, 75, 251, 102, 26];

entrypoint!(process_instruction);

fn process_instruction(
    program_id: &Address,
    accounts: &[AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    // First, check for the 8-byte Transfer Hook Execute sighash.
    // Token-2022 sends this discriminator when invoking the hook.
    if instruction_data.len() >= 8 && instruction_data[0..8] == EXECUTE_SIGHASH {
        // Data after the 8-byte sighash: amount (u64 LE)
        return TransferHookExecute::try_from((&instruction_data[8..], accounts))?
            .process(program_id);
    }

    // Otherwise, use the standard 1-byte discriminator dispatch.
    let (discriminator, data) = instruction_data
        .split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;

    let ix_discriminator = TransferHookInstructionDiscriminators::try_from(*discriminator)?;

    match ix_discriminator {
        TransferHookInstructionDiscriminators::InitExtraAccountMetas => {
            InitExtraAccountMetas::try_from((data, accounts))?.process(program_id)
        }
        TransferHookInstructionDiscriminators::EmitEvent => {
            spiko_events::process_emit_event(accounts, &event_authority_pda::ID)
        }
    }
}
