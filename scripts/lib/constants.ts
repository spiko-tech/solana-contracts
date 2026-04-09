import { address, type Address } from "@solana/kit";

// =================================================================
// Program IDs (from target/deploy/*-keypair.json)
// =================================================================

export const PERMISSION_MANAGER_PROGRAM_ID: Address =
  address("BTZTjmY3i1ZPFkUvZAwD3WzwQFxxLXeaCYYBNjHKuRoz");

export const SPIKO_TOKEN_PROGRAM_ID: Address =
  address("2LKr4wYMkx75hCbrmRCR2iESCWmeDViuSDLxZaZnC4aP");

export const MINTER_PROGRAM_ID: Address =
  address("6jbcB2eNfm1qLXRFd9jJes9yYEUWYafJDjfZ1dobSQ9z");

export const REDEMPTION_PROGRAM_ID: Address =
  address("GZEFPC74n1ifKrsH9vh67qntZ8bqpzpdDrBasGVCUPPo");

export const TRANSFER_HOOK_PROGRAM_ID: Address =
  address("GMpqabVyd98sefjUB3fLhy6HA77xTnGpWy9e7vJs2vtJ");

export const TOKEN_2022_PROGRAM_ID: Address =
  address("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

export const SYSTEM_PROGRAM_ID: Address =
  address("11111111111111111111111111111111");

export const ASSOCIATED_TOKEN_PROGRAM_ID: Address =
  address("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

export const SYSVAR_RENT_PUBKEY: Address =
  address("SysvarRent111111111111111111111111111111111");

// =================================================================
// PDA Seeds
// =================================================================

export const PERMISSION_CONFIG_SEED = new TextEncoder().encode(
  "permission_config"
);
export const USER_PERMISSION_SEED = new TextEncoder().encode("user_perm");

export const TOKEN_CONFIG_SEED = new TextEncoder().encode("token_config");
export const MINT_AUTHORITY_SEED = new TextEncoder().encode("mint_authority");
export const EXTRA_ACCOUNT_METAS_SEED = new TextEncoder().encode(
  "extra-account-metas"
);

export const MINTER_CONFIG_SEED = new TextEncoder().encode("minter_config");
export const DAILY_LIMIT_SEED = new TextEncoder().encode("daily_limit");
export const MINT_OPERATION_SEED = new TextEncoder().encode("mint_op");

export const REDEMPTION_CONFIG_SEED = new TextEncoder().encode(
  "redemption_config"
);
export const TOKEN_MINIMUM_SEED = new TextEncoder().encode("minimum");
export const VAULT_SEED = new TextEncoder().encode("vault");
export const REDEMPTION_OPERATION_SEED = new TextEncoder().encode(
  "redemption_op"
);

// =================================================================
// Instruction Discriminators
// =================================================================

// permission_manager
export const PM_DISC_INITIALIZE = 0;
export const PM_DISC_GRANT_ROLE = 1;
export const PM_DISC_REVOKE_ROLE = 2;

// spiko_token
export const ST_DISC_INITIALIZE = 0;
export const ST_DISC_MINT = 1;
export const ST_DISC_BURN = 2;
export const ST_DISC_TRANSFER = 3;
export const ST_DISC_PAUSE = 4;
export const ST_DISC_UNPAUSE = 5;
export const ST_DISC_REDEEM = 6;
export const ST_DISC_SET_REDEMPTION_CONTRACT = 7;

// spiko_transfer_hook
export const TH_DISC_INIT_EXTRA_ACCOUNT_METAS = 0;

// minter
export const MT_DISC_INITIALIZE = 0;
export const MT_DISC_INITIATE_MINT = 1;
export const MT_DISC_APPROVE_MINT = 2;
export const MT_DISC_SET_DAILY_LIMIT = 4;

// redemption
export const RD_DISC_INITIALIZE = 0;
export const RD_DISC_EXECUTE = 1;
export const RD_DISC_CANCEL = 2;
export const RD_DISC_SET_MINIMUM = 3;
export const RD_DISC_ON_REDEEM = 4;

// =================================================================
// Role Bits
// =================================================================

export const ROLE_MINTER = 0;
export const ROLE_PAUSER = 1;
export const ROLE_BURNER = 2;
export const ROLE_WHITELISTER = 3;
export const ROLE_WHITELISTED = 4;
export const ROLE_REDEMPTION_EXECUTOR = 5;
export const ROLE_MINT_APPROVER = 6;
export const ROLE_MINT_INITIATOR = 7;

export const ALL_ROLES = [
  ROLE_MINTER,
  ROLE_PAUSER,
  ROLE_BURNER,
  ROLE_WHITELISTER,
  ROLE_WHITELISTED,
  ROLE_REDEMPTION_EXECUTOR,
  ROLE_MINT_APPROVER,
  ROLE_MINT_INITIATOR,
] as const;

export const ROLE_NAMES: Record<number, string> = {
  [ROLE_MINTER]: "MINTER",
  [ROLE_PAUSER]: "PAUSER",
  [ROLE_BURNER]: "BURNER",
  [ROLE_WHITELISTER]: "WHITELISTER",
  [ROLE_WHITELISTED]: "WHITELISTED",
  [ROLE_REDEMPTION_EXECUTOR]: "REDEMPTION_EXECUTOR",
  [ROLE_MINT_APPROVER]: "MINT_APPROVER",
  [ROLE_MINT_INITIATOR]: "MINT_INITIATOR",
};

// =================================================================
// Token Constants
// =================================================================

// Token-2022 Mint layout with fixed-size extensions
// (TransferHook + PermanentDelegate + MetadataPointer)
// plus variable-size TokenMetadata extension:
//   base_mint(82) + padding(83) + account_type(1) = 166
//   + TransferHook TLV: ext_type(2) + ext_len(2) + ext_data(64) = 68
//   + PermanentDelegate TLV: ext_type(2) + ext_len(2) + ext_data(32) = 36
//   + MetadataPointer TLV: ext_type(2) + ext_len(2) + ext_data(64) = 68
//   Fixed extensions = 166 + 68 + 36 + 68 = 338
//   + TokenMetadata TLV: ext_type(2) + ext_len(2) + variable metadata data
//   Metadata data = update_authority(32) + mint(32) + name(4+N) + symbol(4+S) + uri(4+U) + additional_metadata(4)
//
// Use mintAccountSize() to compute the exact size for a given token.
// Use MINT_FIXED_EXTENSIONS_SIZE for the CreateAccount space parameter
// (InitializeMint2 requires account size to match fixed extensions only;
// TokenMetadataInitialize will realloc to add the variable-length metadata).
export const MINT_FIXED_EXTENSIONS_SIZE = 338n;

export function mintAccountSize(name: string, symbol: string, uri: string): bigint {
  const fixed = 338;
  const metadataData = 32 + 32 + 4 + name.length + 4 + symbol.length + 4 + uri.length + 4;
  return BigInt(fixed + 4 + metadataData); // +4 for TLV type+len header
}

// =================================================================
// Deployment Parameters
// =================================================================

// Minter: max delay = 1 day (86400 seconds)
export const MAX_DELAY = 86400n;

// Daily limit: 5,000,000 shares × 10^5 decimals = 500,000,000,000
export const DAILY_LIMIT = 500_000_000_000n;

// Redemption minimum: 1 share × 10^5 decimals = 100,000
export const REDEMPTION_MINIMUM = 100_000n;
