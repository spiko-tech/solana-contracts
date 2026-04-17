use codama::CodamaType;
use pinocchio::Address;
use std::vec::Vec;

use crate::discriminators::RedemptionEventDiscriminators;
use spiko_events::{EventDiscriminator, EventSerialize};

#[derive(CodamaType)]
pub struct RedemptionExecutedEvent {
    pub operator: Address,
    pub user: Address,
    pub mint: Address,
    pub amount: u64,
    pub salt: u64,
}

impl EventDiscriminator for RedemptionExecutedEvent {
    const DISCRIMINATOR: u8 = RedemptionEventDiscriminators::RedemptionExecuted as u8;
}

impl EventSerialize for RedemptionExecutedEvent {
    #[inline(always)]
    fn to_bytes_inner(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(Self::DATA_LEN);
        data.extend_from_slice(self.operator.as_ref());
        data.extend_from_slice(self.user.as_ref());
        data.extend_from_slice(self.mint.as_ref());
        data.extend_from_slice(&self.amount.to_le_bytes());
        data.extend_from_slice(&self.salt.to_le_bytes());
        data
    }
}

impl RedemptionExecutedEvent {
    pub const DATA_LEN: usize = 32 + 32 + 32 + 8 + 8;

    #[inline(always)]
    pub fn new(operator: Address, user: Address, mint: Address, amount: u64, salt: u64) -> Self {
        Self {
            operator,
            user,
            mint,
            amount,
            salt,
        }
    }
}
