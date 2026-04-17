use codama::CodamaType;
use pinocchio::Address;
use std::vec::Vec;

use crate::discriminators::TokenEventDiscriminators;
use spiko_events::{EventDiscriminator, EventSerialize};

#[derive(CodamaType)]
pub struct BurnEvent {
    pub caller: Address,
    pub mint: Address,
    pub source_ata: Address,
    pub amount: u64,
}

impl EventDiscriminator for BurnEvent {
    const DISCRIMINATOR: u8 = TokenEventDiscriminators::Burn as u8;
}

impl EventSerialize for BurnEvent {
    #[inline(always)]
    fn to_bytes_inner(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(Self::DATA_LEN);
        data.extend_from_slice(self.caller.as_ref());
        data.extend_from_slice(self.mint.as_ref());
        data.extend_from_slice(self.source_ata.as_ref());
        data.extend_from_slice(&self.amount.to_le_bytes());
        data
    }
}

impl BurnEvent {
    pub const DATA_LEN: usize = 32 + 32 + 32 + 8;

    #[inline(always)]
    pub fn new(caller: Address, mint: Address, source_ata: Address, amount: u64) -> Self {
        Self {
            caller,
            mint,
            source_ata,
            amount,
        }
    }
}
