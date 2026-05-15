pub const GATEKEEPER_CONFIG_SEED: &[u8] = b"gatekeeper_config";
pub const VAULT_AUTHORITY_SEED: &[u8] = b"vault_authority";
pub const WITHDRAWAL_DAILY_LIMIT_SEED: &[u8] = b"withdrawal_limit";
pub const GATEKEEP_OPERATION_SEED: &[u8] = b"gatekeep_op";

pub const SECONDS_PER_DAY: i64 = 86400;

pub const STATUS_PENDING: u8 = 1;
pub const STATUS_DONE: u8 = 2;
pub const STATUS_CANCELED: u8 = 3;
