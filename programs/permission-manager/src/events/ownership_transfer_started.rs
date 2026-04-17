use codama::CodamaType;
use pinocchio::Address;
use std::vec::Vec;

use crate::discriminators::PermissionManagerEventDiscriminators;
use spiko_events::{EventDiscriminator, EventSerialize};

#[derive(CodamaType)]
pub struct OwnershipTransferStartedEvent {
    pub admin: Address,
    pub new_admin: Address,
}

impl EventDiscriminator for OwnershipTransferStartedEvent {
    const DISCRIMINATOR: u8 = PermissionManagerEventDiscriminators::OwnershipTransferStarted as u8;
}

impl EventSerialize for OwnershipTransferStartedEvent {
    #[inline(always)]
    fn to_bytes_inner(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(Self::DATA_LEN);
        data.extend_from_slice(self.admin.as_ref());
        data.extend_from_slice(self.new_admin.as_ref());
        data
    }
}

impl OwnershipTransferStartedEvent {
    pub const DATA_LEN: usize = 32 + 32;

    #[inline(always)]
    pub fn new(admin: Address, new_admin: Address) -> Self {
        Self { admin, new_admin }
    }
}
