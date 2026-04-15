use pinocchio::error::ProgramError;

/// Instruction data for the InitializeToken instruction.
///
/// Data layout (after discriminator byte):
///   [0]        decimals (u8)
///   [1..5]     name_len (u32 LE)
///   [5..5+N]   name (UTF-8 bytes)
///   [5+N..9+N] symbol_len (u32 LE)
///   [9+N..9+N+S] symbol (UTF-8 bytes)
///   [9+N+S..13+N+S] uri_len (u32 LE)
///   [13+N+S..13+N+S+U] uri (UTF-8 bytes)
pub struct InitializeTokenData<'a> {
    pub decimals: u8,
    pub name: &'a [u8],
    pub symbol: &'a [u8],
    pub uri: &'a [u8],
}

impl<'a> TryFrom<&'a [u8]> for InitializeTokenData<'a> {
    type Error = ProgramError;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        if data.is_empty() {
            return Err(ProgramError::InvalidInstructionData);
        }

        let decimals = data[0];
        let mut offset = 1;

        // Parse name
        if data.len() < offset + 4 {
            return Err(ProgramError::InvalidInstructionData);
        }
        let name_len = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;
        if data.len() < offset + name_len {
            return Err(ProgramError::InvalidInstructionData);
        }
        let name = &data[offset..offset + name_len];
        offset += name_len;

        // Parse symbol
        if data.len() < offset + 4 {
            return Err(ProgramError::InvalidInstructionData);
        }
        let symbol_len = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;
        if data.len() < offset + symbol_len {
            return Err(ProgramError::InvalidInstructionData);
        }
        let symbol = &data[offset..offset + symbol_len];
        offset += symbol_len;

        // Parse uri
        if data.len() < offset + 4 {
            return Err(ProgramError::InvalidInstructionData);
        }
        let uri_len = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;
        if data.len() < offset + uri_len {
            return Err(ProgramError::InvalidInstructionData);
        }
        let uri = &data[offset..offset + uri_len];

        Ok(Self {
            decimals,
            name,
            symbol,
            uri,
        })
    }
}
