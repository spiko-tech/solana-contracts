use pinocchio::error::ProgramError;

/// Instruction data for the SetMaxDelay instruction.
///
/// Data layout:
///   [0..8] max_delay (i64, little-endian, seconds)
pub struct SetMaxDelayData {
    pub max_delay: i64,
}

impl<'a> TryFrom<&'a [u8]> for SetMaxDelayData {
    type Error = ProgramError;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        if data.len() < 8 {
            return Err(ProgramError::InvalidInstructionData);
        }
        let max_delay = i64::from_le_bytes(data[0..8].try_into().unwrap());

        Ok(Self { max_delay })
    }
}

impl<'a> spiko_common::InstructionData<'a> for SetMaxDelayData {
    const LEN: usize = 8;
}
