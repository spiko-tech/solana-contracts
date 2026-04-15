import {
  type Address,
  getAddressEncoder,
  getProgramDerivedAddress,
} from "@solana/kit";

import {
  PERMISSION_MANAGER_PROGRAM_ID,
  PERMISSION_CONFIG_SEED,
  USER_PERMISSION_SEED,
  SPIKO_TOKEN_PROGRAM_ID,
  TOKEN_CONFIG_SEED,
  MINT_AUTHORITY_SEED,
  EXTRA_ACCOUNT_METAS_SEED,
  TRANSFER_HOOK_PROGRAM_ID,
  MINTER_PROGRAM_ID,
  MINTER_CONFIG_SEED,
  DAILY_LIMIT_SEED,
  MINT_OPERATION_SEED,
  REDEMPTION_PROGRAM_ID,
  REDEMPTION_CONFIG_SEED,
  TOKEN_MINIMUM_SEED,
  VAULT_SEED,
  REDEMPTION_OPERATION_SEED,
  EVENT_AUTHORITY_SEED,
} from "./constants.js";

const addressEncoder = getAddressEncoder();

// =================================================================
// Permission Manager PDAs
// =================================================================

export async function permissionConfigPda(): Promise<[Address, number]> {
  const [addr, bump] = await getProgramDerivedAddress({
    programAddress: PERMISSION_MANAGER_PROGRAM_ID,
    seeds: [PERMISSION_CONFIG_SEED],
  });
  return [addr, bump];
}

export async function userPermissionsPda(
  user: Address
): Promise<[Address, number]> {
  const [addr, bump] = await getProgramDerivedAddress({
    programAddress: PERMISSION_MANAGER_PROGRAM_ID,
    seeds: [USER_PERMISSION_SEED, addressEncoder.encode(user)],
  });
  return [addr, bump];
}

// =================================================================
// Spiko Token PDAs
// =================================================================

export async function tokenConfigPda(
  mint: Address
): Promise<[Address, number]> {
  const [addr, bump] = await getProgramDerivedAddress({
    programAddress: SPIKO_TOKEN_PROGRAM_ID,
    seeds: [TOKEN_CONFIG_SEED, addressEncoder.encode(mint)],
  });
  return [addr, bump];
}

export async function mintAuthorityPda(
  mint: Address
): Promise<[Address, number]> {
  const [addr, bump] = await getProgramDerivedAddress({
    programAddress: SPIKO_TOKEN_PROGRAM_ID,
    seeds: [MINT_AUTHORITY_SEED, addressEncoder.encode(mint)],
  });
  return [addr, bump];
}

export async function extraAccountMetaListPda(
  mint: Address
): Promise<[Address, number]> {
  const [addr, bump] = await getProgramDerivedAddress({
    programAddress: TRANSFER_HOOK_PROGRAM_ID,
    seeds: [EXTRA_ACCOUNT_METAS_SEED, addressEncoder.encode(mint)],
  });
  return [addr, bump];
}

// =================================================================
// Minter PDAs
// =================================================================

export async function minterConfigPda(): Promise<[Address, number]> {
  const [addr, bump] = await getProgramDerivedAddress({
    programAddress: MINTER_PROGRAM_ID,
    seeds: [MINTER_CONFIG_SEED],
  });
  return [addr, bump];
}

export async function dailyLimitPda(
  mint: Address
): Promise<[Address, number]> {
  const [addr, bump] = await getProgramDerivedAddress({
    programAddress: MINTER_PROGRAM_ID,
    seeds: [DAILY_LIMIT_SEED, addressEncoder.encode(mint)],
  });
  return [addr, bump];
}

export async function mintOperationPda(
  operationId: Uint8Array
): Promise<[Address, number]> {
  const [addr, bump] = await getProgramDerivedAddress({
    programAddress: MINTER_PROGRAM_ID,
    seeds: [MINT_OPERATION_SEED, operationId],
  });
  return [addr, bump];
}

// =================================================================
// Redemption PDAs
// =================================================================

export async function redemptionConfigPda(): Promise<[Address, number]> {
  const [addr, bump] = await getProgramDerivedAddress({
    programAddress: REDEMPTION_PROGRAM_ID,
    seeds: [REDEMPTION_CONFIG_SEED],
  });
  return [addr, bump];
}

export async function tokenMinimumPda(
  mint: Address
): Promise<[Address, number]> {
  const [addr, bump] = await getProgramDerivedAddress({
    programAddress: REDEMPTION_PROGRAM_ID,
    seeds: [TOKEN_MINIMUM_SEED, addressEncoder.encode(mint)],
  });
  return [addr, bump];
}

export async function vaultAuthorityPda(): Promise<[Address, number]> {
  const [addr, bump] = await getProgramDerivedAddress({
    programAddress: REDEMPTION_PROGRAM_ID,
    seeds: [VAULT_SEED],
  });
  return [addr, bump];
}

export async function redemptionOperationPda(
  operationId: Uint8Array
): Promise<[Address, number]> {
  const [addr, bump] = await getProgramDerivedAddress({
    programAddress: REDEMPTION_PROGRAM_ID,
    seeds: [REDEMPTION_OPERATION_SEED, operationId],
  });
  return [addr, bump];
}

// =================================================================
// Event Authority PDAs (self-CPI event emission)
//
// Each program derives its event_authority PDA from seeds=["event_authority"]
// using its own program ID.
// =================================================================

export async function permissionManagerEventAuthorityPda(): Promise<[Address, number]> {
  const [addr, bump] = await getProgramDerivedAddress({
    programAddress: PERMISSION_MANAGER_PROGRAM_ID,
    seeds: [EVENT_AUTHORITY_SEED],
  });
  return [addr, bump];
}

export async function spikoTokenEventAuthorityPda(): Promise<[Address, number]> {
  const [addr, bump] = await getProgramDerivedAddress({
    programAddress: SPIKO_TOKEN_PROGRAM_ID,
    seeds: [EVENT_AUTHORITY_SEED],
  });
  return [addr, bump];
}

export async function transferHookEventAuthorityPda(): Promise<[Address, number]> {
  const [addr, bump] = await getProgramDerivedAddress({
    programAddress: TRANSFER_HOOK_PROGRAM_ID,
    seeds: [EVENT_AUTHORITY_SEED],
  });
  return [addr, bump];
}

export async function minterEventAuthorityPda(): Promise<[Address, number]> {
  const [addr, bump] = await getProgramDerivedAddress({
    programAddress: MINTER_PROGRAM_ID,
    seeds: [EVENT_AUTHORITY_SEED],
  });
  return [addr, bump];
}

export async function redemptionEventAuthorityPda(): Promise<[Address, number]> {
  const [addr, bump] = await getProgramDerivedAddress({
    programAddress: REDEMPTION_PROGRAM_ID,
    seeds: [EVENT_AUTHORITY_SEED],
  });
  return [addr, bump];
}
