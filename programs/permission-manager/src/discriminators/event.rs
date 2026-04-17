/// Event discriminators for the Permission Manager program.
#[repr(u8)]
pub enum PermissionManagerEventDiscriminators {
    Initialized = 0,
    RoleGranted = 1,
    RoleRemoved = 2,
    OwnershipTransferStarted = 3,
    OwnershipTransferred = 4,
}
