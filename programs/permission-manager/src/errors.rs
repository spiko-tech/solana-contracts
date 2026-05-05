use anchor_lang::prelude::*;

#[error_code]
pub enum PermissionError {
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Invalid role combination")]
    InvalidRole,
    #[msg("Whitelister can only grant WHITELISTED or WHITELISTED_EXT")]
    RoleNotWhitelistManageable,
    #[msg("Cannot grant WHITELISTED and WHITELISTED_EXT to the same address")]
    MutualExclusionViolation,
}
