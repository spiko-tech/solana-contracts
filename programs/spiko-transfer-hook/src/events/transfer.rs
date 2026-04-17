use codama::CodamaType;
use pinocchio::Address;
use std::vec::Vec;

use crate::discriminators::TransferHookEventDiscriminators;
use spiko_events::{EventDiscriminator, EventSerialize};

#[derive(CodamaType)]
pub struct TransferEvent {
    pub sender: Address,
    pub mint: Address,
    pub source: Address,
    pub destination: Address,
    pub amount: u64,
}

impl EventDiscriminator for TransferEvent {
    const DISCRIMINATOR: u8 = TransferHookEventDiscriminators::Transfer as u8;
}

impl EventSerialize for TransferEvent {
    #[inline(always)]
    fn to_bytes_inner(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(Self::DATA_LEN);
        data.extend_from_slice(self.sender.as_ref());
        data.extend_from_slice(self.mint.as_ref());
        data.extend_from_slice(self.source.as_ref());
        data.extend_from_slice(self.destination.as_ref());
        data.extend_from_slice(&self.amount.to_le_bytes());
        data
    }
}

impl TransferEvent {
    pub const DATA_LEN: usize = 32 + 32 + 32 + 32 + 8;

    #[inline(always)]
    pub fn new(
        sender: Address,
        mint: Address,
        source: Address,
        destination: Address,
        amount: u64,
    ) -> Self {
        Self {
            sender,
            mint,
            source,
            destination,
            amount,
        }
    }
}
