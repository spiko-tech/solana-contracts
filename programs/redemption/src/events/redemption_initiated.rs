use codama::CodamaType;
use pinocchio::Address;
use std::vec::Vec;

use crate::discriminators::RedemptionEventDiscriminators;
use spiko_events::{EventDiscriminator, EventSerialize};

#[derive(CodamaType)]
pub struct RedemptionInitiatedEvent {
    pub user: Address,
    pub mint: Address,
    pub amount: u64,
    pub salt: u64,
    pub deadline: i64,
}

impl EventDiscriminator for RedemptionInitiatedEvent {
    const DISCRIMINATOR: u8 = RedemptionEventDiscriminators::RedemptionInitiated as u8;
}

impl EventSerialize for RedemptionInitiatedEvent {
    #[inline(always)]
    fn to_bytes_inner(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(Self::DATA_LEN);
        data.extend_from_slice(self.user.as_ref());
        data.extend_from_slice(self.mint.as_ref());
        data.extend_from_slice(&self.amount.to_le_bytes());
        data.extend_from_slice(&self.salt.to_le_bytes());
        data.extend_from_slice(&self.deadline.to_le_bytes());
        data
    }
}

impl RedemptionInitiatedEvent {
    pub const DATA_LEN: usize = 32 + 32 + 8 + 8 + 8;

    #[inline(always)]
    pub fn new(user: Address, mint: Address, amount: u64, salt: u64, deadline: i64) -> Self {
        Self {
            user,
            mint,
            amount,
            salt,
            deadline,
        }
    }
}
