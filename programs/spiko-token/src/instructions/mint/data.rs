use pinocchio::error::ProgramError;

/// Instruction data for the MintToken instruction.
///
/// Data layout:
///   [0..8] amount (u64, little-endian)
pub struct MintTokenData {
    pub amount: u64,
}

impl<'a> TryFrom<&'a [u8]> for MintTokenData {
    type Error = ProgramError;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        if data.len() < 8 {
            return Err(ProgramError::InvalidInstructionData);
        }
        let amount = u64::from_le_bytes(data[0..8].try_into().unwrap());

        Ok(Self { amount })
    }
}
