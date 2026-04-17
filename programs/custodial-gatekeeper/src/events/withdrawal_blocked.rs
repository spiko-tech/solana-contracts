use codama::CodamaType;
use pinocchio::Address;
use std::vec::Vec;

use crate::discriminators::GatekeeperEventDiscriminators;
use spiko_events::{EventDiscriminator, EventSerialize};

#[derive(CodamaType)]
pub struct WithdrawalBlockedEvent {
    pub sender: Address,
    pub recipient: Address,
    pub mint: Address,
    pub amount: u64,
    pub salt: u64,
    pub deadline: i64,
}

impl EventDiscriminator for WithdrawalBlockedEvent {
    const DISCRIMINATOR: u8 = GatekeeperEventDiscriminators::WithdrawalBlocked as u8;
}

impl EventSerialize for WithdrawalBlockedEvent {
    #[inline(always)]
    fn to_bytes_inner(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(Self::DATA_LEN);
        data.extend_from_slice(self.sender.as_ref());
        data.extend_from_slice(self.recipient.as_ref());
        data.extend_from_slice(self.mint.as_ref());
        data.extend_from_slice(&self.amount.to_le_bytes());
        data.extend_from_slice(&self.salt.to_le_bytes());
        data.extend_from_slice(&self.deadline.to_le_bytes());
        data
    }
}

impl WithdrawalBlockedEvent {
    pub const DATA_LEN: usize = 32 + 32 + 32 + 8 + 8 + 8;

    #[inline(always)]
    pub fn new(
        sender: Address,
        recipient: Address,
        mint: Address,
        amount: u64,
        salt: u64,
        deadline: i64,
    ) -> Self {
        Self {
            sender,
            recipient,
            mint,
            amount,
            salt,
            deadline,
        }
    }
}
