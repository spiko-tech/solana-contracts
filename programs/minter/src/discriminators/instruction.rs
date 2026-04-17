use pinocchio::error::ProgramError;

/// Discriminators for the Minter program instructions.
#[repr(u8)]
pub enum MinterInstructionDiscriminators {
    InitializeMinter = 0,
    InitiateMint = 1,
    ApproveMint = 2,
    CancelMint = 3,
    SetDailyLimit = 4,
    SetMaxDelay = 5,
    EmitEvent = 255,
}

impl TryFrom<u8> for MinterInstructionDiscriminators {
    type Error = ProgramError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::InitializeMinter),
            1 => Ok(Self::InitiateMint),
            2 => Ok(Self::ApproveMint),
            3 => Ok(Self::CancelMint),
            4 => Ok(Self::SetDailyLimit),
            5 => Ok(Self::SetMaxDelay),
            255 => Ok(Self::EmitEvent),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
