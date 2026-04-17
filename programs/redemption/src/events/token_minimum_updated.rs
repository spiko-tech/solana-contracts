use codama::CodamaType;
use pinocchio::Address;
use std::vec::Vec;

use crate::discriminators::RedemptionEventDiscriminators;
use spiko_events::{EventDiscriminator, EventSerialize};

#[derive(CodamaType)]
pub struct TokenMinimumUpdatedEvent {
    pub caller: Address,
    pub mint: Address,
    pub minimum: u64,
}

impl EventDiscriminator for TokenMinimumUpdatedEvent {
    const DISCRIMINATOR: u8 = RedemptionEventDiscriminators::TokenMinimumUpdated as u8;
}

impl EventSerialize for TokenMinimumUpdatedEvent {
    #[inline(always)]
    fn to_bytes_inner(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(Self::DATA_LEN);
        data.extend_from_slice(self.caller.as_ref());
        data.extend_from_slice(self.mint.as_ref());
        data.extend_from_slice(&self.minimum.to_le_bytes());
        data
    }
}

impl TokenMinimumUpdatedEvent {
    pub const DATA_LEN: usize = 32 + 32 + 8;

    #[inline(always)]
    pub fn new(caller: Address, mint: Address, minimum: u64) -> Self {
        Self {
            caller,
            mint,
            minimum,
        }
    }
}
