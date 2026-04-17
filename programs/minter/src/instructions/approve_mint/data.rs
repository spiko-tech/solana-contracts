use pinocchio::error::ProgramError;

/// Instruction data for the ApproveMint instruction.
///
/// Data layout:
///   [0..32]  user/recipient address (32 bytes)
///   [32..64] token_mint address (32 bytes) — needed to recompute operation_id
///   [64..72] amount (u64, little-endian)
///   [72..80] salt (u64, little-endian)
pub struct ApproveMintData {
    pub user: [u8; 32],
    pub token_mint_key: [u8; 32],
    pub amount: u64,
    pub salt: u64,
}

impl<'a> TryFrom<&'a [u8]> for ApproveMintData {
    type Error = ProgramError;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        if data.len() < 80 {
            return Err(ProgramError::InvalidInstructionData);
        }

        let mut user = [0u8; 32];
        user.copy_from_slice(&data[0..32]);
        let mut token_mint_key = [0u8; 32];
        token_mint_key.copy_from_slice(&data[32..64]);
        let amount = u64::from_le_bytes(data[64..72].try_into().unwrap());
        let salt = u64::from_le_bytes(data[72..80].try_into().unwrap());

        Ok(Self {
            user,
            token_mint_key,
            amount,
            salt,
        })
    }
}

impl<'a> spiko_common::InstructionData<'a> for ApproveMintData {
    const LEN: usize = 80;
}
