use pinocchio::error::ProgramError;

/// Discriminators for the Redemption program instructions.
#[repr(u8)]
pub enum RedemptionInstructionDiscriminators {
    InitializeRedemption = 0,
    ExecuteRedemption = 1,
    CancelRedemption = 2,
    SetMinimum = 3,
    OnRedeem = 4,
    EmitEvent = 255,
}

impl TryFrom<u8> for RedemptionInstructionDiscriminators {
    type Error = ProgramError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::InitializeRedemption),
            1 => Ok(Self::ExecuteRedemption),
            2 => Ok(Self::CancelRedemption),
            3 => Ok(Self::SetMinimum),
            4 => Ok(Self::OnRedeem),
            255 => Ok(Self::EmitEvent),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
