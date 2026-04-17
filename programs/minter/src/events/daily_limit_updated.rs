use codama::CodamaType;
use pinocchio::Address;
use std::vec::Vec;

use crate::discriminators::MinterEventDiscriminators;
use spiko_events::{EventDiscriminator, EventSerialize};

#[derive(CodamaType)]
pub struct DailyLimitUpdatedEvent {
    pub caller: Address,
    pub mint: Address,
    pub limit: u64,
}

impl EventDiscriminator for DailyLimitUpdatedEvent {
    const DISCRIMINATOR: u8 = MinterEventDiscriminators::DailyLimitUpdated as u8;
}

impl EventSerialize for DailyLimitUpdatedEvent {
    #[inline(always)]
    fn to_bytes_inner(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(Self::DATA_LEN);
        data.extend_from_slice(self.caller.as_ref());
        data.extend_from_slice(self.mint.as_ref());
        data.extend_from_slice(&self.limit.to_le_bytes());
        data
    }
}

impl DailyLimitUpdatedEvent {
    pub const DATA_LEN: usize = 32 + 32 + 8;

    #[inline(always)]
    pub fn new(caller: Address, mint: Address, limit: u64) -> Self {
        Self {
            caller,
            mint,
            limit,
        }
    }
}
