use pinocchio::error::ProgramError;

/// Discriminators for the CustodialGatekeeper program instructions.
#[repr(u8)]
pub enum GatekeeperInstructionDiscriminators {
    Initialize = 0,
    SetDailyLimit = 1,
    CustodialWithdraw = 2,
    ApproveWithdrawal = 3,
    CancelWithdrawal = 4,
    EmitEvent = 255,
}

impl TryFrom<u8> for GatekeeperInstructionDiscriminators {
    type Error = ProgramError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Initialize),
            1 => Ok(Self::SetDailyLimit),
            2 => Ok(Self::CustodialWithdraw),
            3 => Ok(Self::ApproveWithdrawal),
            4 => Ok(Self::CancelWithdrawal),
            255 => Ok(Self::EmitEvent),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
