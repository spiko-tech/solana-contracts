use pinocchio::error::ProgramError;

/// Instruction data for the SetRedemptionContract instruction.
///
/// Data layout:
///   [0..32]  redemption_contract address (32 bytes; all zeros to clear)
pub struct SetRedemptionContractData {
    pub redemption_contract: [u8; 32],
}

impl<'a> TryFrom<&'a [u8]> for SetRedemptionContractData {
    type Error = ProgramError;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        if data.len() < 32 {
            return Err(ProgramError::InvalidInstructionData);
        }

        let mut redemption_contract = [0u8; 32];
        redemption_contract.copy_from_slice(&data[0..32]);

        Ok(Self {
            redemption_contract,
        })
    }
}
