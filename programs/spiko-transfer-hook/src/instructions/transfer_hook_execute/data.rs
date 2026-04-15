use pinocchio::error::ProgramError;

/// Instruction data for the TransferHookExecute instruction.
///
/// Data (from Token-2022, after 8-byte sighash):
///   [0..8] amount (u64, little-endian)
pub struct TransferHookExecuteData {
    pub amount: u64,
}

impl<'a> TryFrom<&'a [u8]> for TransferHookExecuteData {
    type Error = ProgramError;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        let amount = if data.len() >= 8 {
            u64::from_le_bytes(data[0..8].try_into().unwrap())
        } else {
            0
        };

        Ok(Self { amount })
    }
}
