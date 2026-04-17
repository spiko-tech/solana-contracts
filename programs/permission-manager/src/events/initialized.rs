use codama::CodamaType;
use pinocchio::Address;
use std::vec::Vec;

use crate::discriminators::PermissionManagerEventDiscriminators;
use spiko_events::{EventDiscriminator, EventSerialize};

#[derive(CodamaType)]
pub struct PermissionManagerInitializedEvent {
    pub admin: Address,
}

impl EventDiscriminator for PermissionManagerInitializedEvent {
    const DISCRIMINATOR: u8 = PermissionManagerEventDiscriminators::Initialized as u8;
}

impl EventSerialize for PermissionManagerInitializedEvent {
    #[inline(always)]
    fn to_bytes_inner(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(Self::DATA_LEN);
        data.extend_from_slice(self.admin.as_ref());
        data
    }
}

impl PermissionManagerInitializedEvent {
    pub const DATA_LEN: usize = 32;

    #[inline(always)]
    pub fn new(admin: Address) -> Self {
        Self { admin }
    }
}
