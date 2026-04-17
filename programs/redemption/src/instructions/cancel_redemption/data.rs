use pinocchio::error::ProgramError;

use spiko_common::InstructionData;

/// Instruction data for the CancelRedemption instruction.
///
/// Data layout:
///   [0..32]  user address (32 bytes)
///   [32..40] amount (u64, little-endian)
///   [40..48] salt (u64, little-endian)
pub struct CancelRedemptionData {
    pub user: [u8; 32],
    pub amount: u64,
    pub salt: u64,
}

impl<'a> TryFrom<&'a [u8]> for CancelRedemptionData {
    type Error = ProgramError;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        if data.len() < 48 {
            return Err(ProgramError::InvalidInstructionData);
        }

        let mut user = [0u8; 32];
        user.copy_from_slice(&data[0..32]);
        let amount = u64::from_le_bytes(data[32..40].try_into().unwrap());
        let salt = u64::from_le_bytes(data[40..48].try_into().unwrap());

        Ok(Self { user, amount, salt })
    }
}

impl<'a> InstructionData<'a> for CancelRedemptionData {
    const LEN: usize = 48;
}
