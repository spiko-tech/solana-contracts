use pinocchio::error::ProgramError;

/// Instruction data for the RedeemToken instruction.
///
/// Data layout:
///   [0..8]  amount (u64, little-endian)
///   [8..16] salt (u64, little-endian)
pub struct RedeemTokenData {
    pub amount: u64,
    pub salt: u64,
}

impl<'a> TryFrom<&'a [u8]> for RedeemTokenData {
    type Error = ProgramError;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        // 8 + 8 = 16 bytes
        if data.len() < 16 {
            return Err(ProgramError::InvalidInstructionData);
        }

        let amount = u64::from_le_bytes(data[0..8].try_into().unwrap());
        let salt = u64::from_le_bytes(data[8..16].try_into().unwrap());

        Ok(Self { amount, salt })
    }
}
