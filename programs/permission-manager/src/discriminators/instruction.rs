use pinocchio::error::ProgramError;

/// Discriminators for the Permission Manager program instructions.
#[repr(u8)]
pub enum PermissionManagerInstructionDiscriminators {
    Initialize = 0,
    GrantRole = 1,
    RevokeRole = 2,
    TransferOwnership = 3,
    AcceptOwnership = 4,
    EmitEvent = 255,
}

impl TryFrom<u8> for PermissionManagerInstructionDiscriminators {
    type Error = ProgramError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Initialize),
            1 => Ok(Self::GrantRole),
            2 => Ok(Self::RevokeRole),
            3 => Ok(Self::TransferOwnership),
            4 => Ok(Self::AcceptOwnership),
            255 => Ok(Self::EmitEvent),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
