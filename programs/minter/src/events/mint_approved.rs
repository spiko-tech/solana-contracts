use codama::CodamaType;
use pinocchio::Address;
use std::vec::Vec;

use crate::discriminators::MinterEventDiscriminators;
use spiko_events::{EventDiscriminator, EventSerialize};

#[derive(CodamaType)]
pub struct MintApprovedEvent {
    pub approver: Address,
    pub user: Address,
    pub mint: Address,
    pub amount: u64,
    pub salt: u64,
}

impl EventDiscriminator for MintApprovedEvent {
    const DISCRIMINATOR: u8 = MinterEventDiscriminators::MintApproved as u8;
}

impl EventSerialize for MintApprovedEvent {
    #[inline(always)]
    fn to_bytes_inner(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(Self::DATA_LEN);
        data.extend_from_slice(self.approver.as_ref());
        data.extend_from_slice(self.user.as_ref());
        data.extend_from_slice(self.mint.as_ref());
        data.extend_from_slice(&self.amount.to_le_bytes());
        data.extend_from_slice(&self.salt.to_le_bytes());
        data
    }
}

impl MintApprovedEvent {
    pub const DATA_LEN: usize = 32 + 32 + 32 + 8 + 8;

    #[inline(always)]
    pub fn new(approver: Address, user: Address, mint: Address, amount: u64, salt: u64) -> Self {
        Self {
            approver,
            user,
            mint,
            amount,
            salt,
        }
    }
}
