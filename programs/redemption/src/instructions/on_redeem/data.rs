use pinocchio::error::ProgramError;

/// Instruction data for the OnRedeem instruction.
///
/// Data layout (48 bytes, after discriminator is stripped by dispatch):
///   [0..32]  user address (32 bytes)
///   [32..40] amount (u64, little-endian)
///   [40..48] salt (u64, little-endian)
pub struct OnRedeemData {
    pub user_address: [u8; 32],
    pub amount: u64,
    pub salt: u64,
}

impl<'a> TryFrom<&'a [u8]> for OnRedeemData {
    type Error = ProgramError;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        // 32 + 8 + 8 = 48 bytes
        if data.len() < 48 {
            return Err(ProgramError::InvalidInstructionData);
        }

        let mut user_address = [0u8; 32];
        user_address.copy_from_slice(&data[0..32]);
        let amount = u64::from_le_bytes(data[32..40].try_into().unwrap());
        let salt = u64::from_le_bytes(data[40..48].try_into().unwrap());

        Ok(Self {
            user_address,
            amount,
            salt,
        })
    }
}
