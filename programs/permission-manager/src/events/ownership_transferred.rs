use codama::CodamaType;
use pinocchio::Address;
use std::vec::Vec;

use crate::discriminators::PermissionManagerEventDiscriminators;
use spiko_events::{EventDiscriminator, EventSerialize};

#[derive(CodamaType)]
pub struct OwnershipTransferredEvent {
    pub new_admin: Address,
}

impl EventDiscriminator for OwnershipTransferredEvent {
    const DISCRIMINATOR: u8 = PermissionManagerEventDiscriminators::OwnershipTransferred as u8;
}

impl EventSerialize for OwnershipTransferredEvent {
    #[inline(always)]
    fn to_bytes_inner(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(Self::DATA_LEN);
        data.extend_from_slice(self.new_admin.as_ref());
        data
    }
}

impl OwnershipTransferredEvent {
    pub const DATA_LEN: usize = 32;

    #[inline(always)]
    pub fn new(new_admin: Address) -> Self {
        Self { new_admin }
    }
}
