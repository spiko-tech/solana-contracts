use pinocchio::error::ProgramError;

/// Instruction data for the RevokeRole instruction.
///
/// Data layout:
///   [0] role_id (u8)
pub struct RevokeRoleData {
    pub role_id: u8,
}

impl<'a> TryFrom<&'a [u8]> for RevokeRoleData {
    type Error = ProgramError;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        if data.is_empty() {
            return Err(ProgramError::InvalidInstructionData);
        }

        Ok(Self { role_id: data[0] })
    }
}
