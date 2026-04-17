use pinocchio::error::ProgramError;

use spiko_common::InstructionData;

/// Instruction data for the SetDailyLimit instruction.
///
/// Data layout:
///   [0..32]  token mint address (32 bytes)
///   [32..40] limit (u64, little-endian)
pub struct SetDailyLimitData {
    pub token_mint: [u8; 32],
    pub limit: u64,
}

impl<'a> TryFrom<&'a [u8]> for SetDailyLimitData {
    type Error = ProgramError;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        if data.len() < 40 {
            return Err(ProgramError::InvalidInstructionData);
        }

        let mut token_mint = [0u8; 32];
        token_mint.copy_from_slice(&data[0..32]);
        let limit = u64::from_le_bytes(data[32..40].try_into().unwrap());

        Ok(Self { token_mint, limit })
    }
}

impl<'a> InstructionData<'a> for SetDailyLimitData {
    const LEN: usize = 40;
}
