use pinocchio::error::ProgramError;

use spiko_common::InstructionData;

/// Instruction data for the Initialize instruction.
///
/// Data layout:
///   [0..32]  permission_manager program ID (32 bytes)
///   [32..40] max_delay (i64, little-endian, seconds)
pub struct InitializeData {
    pub permission_manager: [u8; 32],
    pub max_delay: i64,
}

impl<'a> TryFrom<&'a [u8]> for InitializeData {
    type Error = ProgramError;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        if data.len() < 40 {
            return Err(ProgramError::InvalidInstructionData);
        }
        let mut permission_manager = [0u8; 32];
        permission_manager.copy_from_slice(&data[0..32]);
        let max_delay = i64::from_le_bytes(data[32..40].try_into().unwrap());

        Ok(Self {
            permission_manager,
            max_delay,
        })
    }
}

impl<'a> InstructionData<'a> for InitializeData {
    const LEN: usize = 40;
}
