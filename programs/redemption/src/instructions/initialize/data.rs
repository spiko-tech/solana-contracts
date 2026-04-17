use pinocchio::error::ProgramError;

use spiko_common::InstructionData;

/// Instruction data for the InitializeRedemption instruction.
///
/// Data layout:
///   [0..32] permission_manager program ID (32 bytes)
pub struct InitializeRedemptionData {
    pub permission_manager: [u8; 32],
}

impl<'a> TryFrom<&'a [u8]> for InitializeRedemptionData {
    type Error = ProgramError;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        if data.len() < 32 {
            return Err(ProgramError::InvalidInstructionData);
        }
        let mut permission_manager = [0u8; 32];
        permission_manager.copy_from_slice(&data[0..32]);

        Ok(Self { permission_manager })
    }
}

impl<'a> InstructionData<'a> for InitializeRedemptionData {
    const LEN: usize = 32;
}
