use pinocchio::error::ProgramError;

/// Discriminators for the Spiko Token program instructions.
#[repr(u8)]
pub enum TokenInstructionDiscriminators {
    InitializeToken = 0,
    MintToken = 1,
    BurnToken = 2,
    TransferToken = 3,
    Pause = 4,
    Unpause = 5,
    RedeemToken = 6,
    SetRedemptionContract = 7,
    BurnFrom = 8,
    EmitEvent = 255,
}

impl TryFrom<u8> for TokenInstructionDiscriminators {
    type Error = ProgramError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::InitializeToken),
            1 => Ok(Self::MintToken),
            2 => Ok(Self::BurnToken),
            3 => Ok(Self::TransferToken),
            4 => Ok(Self::Pause),
            5 => Ok(Self::Unpause),
            6 => Ok(Self::RedeemToken),
            7 => Ok(Self::SetRedemptionContract),
            8 => Ok(Self::BurnFrom),
            255 => Ok(Self::EmitEvent),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
