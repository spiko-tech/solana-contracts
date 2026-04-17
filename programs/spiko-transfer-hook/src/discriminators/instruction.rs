use pinocchio::error::ProgramError;

/// Discriminators for the Spiko Transfer Hook program instructions.
///
/// Note: `TransferHookExecute` is dispatched by 8-byte sighash, not by this enum.
#[repr(u8)]
pub enum TransferHookInstructionDiscriminators {
    InitExtraAccountMetas = 0,
    EmitEvent = 255,
}

impl TryFrom<u8> for TransferHookInstructionDiscriminators {
    type Error = ProgramError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::InitExtraAccountMetas),
            255 => Ok(Self::EmitEvent),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
