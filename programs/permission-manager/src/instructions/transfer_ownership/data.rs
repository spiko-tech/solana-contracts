use pinocchio::{address::Address, error::ProgramError};

/// Instruction data for the TransferOwnership instruction.
///
/// Data layout:
///   [0..32] new_admin address (32 bytes)
pub struct TransferOwnershipData {
    pub new_admin: Address,
}

impl<'a> TryFrom<&'a [u8]> for TransferOwnershipData {
    type Error = ProgramError;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        if data.len() < 32 {
            return Err(ProgramError::InvalidInstructionData);
        }

        let mut addr_bytes = [0u8; 32];
        addr_bytes.copy_from_slice(&data[0..32]);
        let new_admin = Address::new_from_array(addr_bytes);

        Ok(Self { new_admin })
    }
}

impl<'a> spiko_common::InstructionData<'a> for TransferOwnershipData {
    const LEN: usize = 32;
}
