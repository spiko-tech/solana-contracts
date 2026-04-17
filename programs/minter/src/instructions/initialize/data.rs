use pinocchio::error::ProgramError;

/// Instruction data for the InitializeMinter instruction.
///
/// Data layout:
///   [0..8]   max_delay (i64, little-endian, seconds)
///   [8..40]  permission_manager program ID (32 bytes)
pub struct InitializeMinterData {
    pub max_delay: i64,
    pub permission_manager: [u8; 32],
}

impl<'a> TryFrom<&'a [u8]> for InitializeMinterData {
    type Error = ProgramError;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        if data.len() < 40 {
            return Err(ProgramError::InvalidInstructionData);
        }
        let max_delay = i64::from_le_bytes(data[0..8].try_into().unwrap());
        let mut permission_manager = [0u8; 32];
        permission_manager.copy_from_slice(&data[8..40]);

        Ok(Self {
            max_delay,
            permission_manager,
        })
    }
}

impl<'a> spiko_common::InstructionData<'a> for InitializeMinterData {
    const LEN: usize = 40;
}
