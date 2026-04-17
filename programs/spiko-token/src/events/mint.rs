use codama::CodamaType;
use pinocchio::Address;
use std::vec::Vec;

use crate::discriminators::TokenEventDiscriminators;
use spiko_events::{EventDiscriminator, EventSerialize};

#[derive(CodamaType)]
pub struct MintEvent {
    pub caller: Address,
    pub mint: Address,
    pub recipient_ata: Address,
    pub amount: u64,
}

impl EventDiscriminator for MintEvent {
    const DISCRIMINATOR: u8 = TokenEventDiscriminators::Mint as u8;
}

impl EventSerialize for MintEvent {
    #[inline(always)]
    fn to_bytes_inner(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(Self::DATA_LEN);
        data.extend_from_slice(self.caller.as_ref());
        data.extend_from_slice(self.mint.as_ref());
        data.extend_from_slice(self.recipient_ata.as_ref());
        data.extend_from_slice(&self.amount.to_le_bytes());
        data
    }
}

impl MintEvent {
    pub const DATA_LEN: usize = 32 + 32 + 32 + 8;

    #[inline(always)]
    pub fn new(caller: Address, mint: Address, recipient_ata: Address, amount: u64) -> Self {
        Self {
            caller,
            mint,
            recipient_ata,
            amount,
        }
    }
}
