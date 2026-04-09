import {
  type Address,
  AccountRole,
  getAddressEncoder,
  type TransactionSigner,
  type ReadonlyUint8Array,
} from "@solana/kit";

import {
  PERMISSION_MANAGER_PROGRAM_ID,
  SPIKO_TOKEN_PROGRAM_ID,
  MINTER_PROGRAM_ID,
  REDEMPTION_PROGRAM_ID,
  TOKEN_2022_PROGRAM_ID,
  SYSTEM_PROGRAM_ID,
  TRANSFER_HOOK_PROGRAM_ID,
  PM_DISC_INITIALIZE,
  PM_DISC_GRANT_ROLE,
  ST_DISC_INITIALIZE,
  ST_DISC_MINT,
  ST_DISC_BURN,
  ST_DISC_TRANSFER,
  ST_DISC_SET_REDEMPTION_CONTRACT,
  TH_DISC_INIT_EXTRA_ACCOUNT_METAS,
  MT_DISC_INITIALIZE,
  MT_DISC_INITIATE_MINT,
  MT_DISC_SET_DAILY_LIMIT,
  RD_DISC_INITIALIZE,
  RD_DISC_EXECUTE,
  RD_DISC_CANCEL,
  RD_DISC_SET_MINIMUM,
  ST_DISC_REDEEM,
} from "./constants.js";

const addressEncoder = getAddressEncoder();

// =================================================================
// Helpers
// =================================================================

/** Encode a bigint as little-endian i64 (8 bytes). */
function encodeI64(value: bigint): Uint8Array {
  const buf = new ArrayBuffer(8);
  new DataView(buf).setBigInt64(0, value, true);
  return new Uint8Array(buf);
}

/** Encode a bigint as little-endian u64 (8 bytes). */
function encodeU64(value: bigint): Uint8Array {
  const buf = new ArrayBuffer(8);
  new DataView(buf).setBigUint64(0, value, true);
  return new Uint8Array(buf);
}

/** Encode an Address as 32 raw bytes. */
function encodeAddress(addr: Address): ReadonlyUint8Array {
  return addressEncoder.encode(addr);
}

/** Concatenate multiple byte arrays. */
function concat(...arrays: (Uint8Array | ReadonlyUint8Array)[]): Uint8Array {
  const totalLen = arrays.reduce((sum, a) => sum + a.length, 0);
  const result = new Uint8Array(totalLen);
  let offset = 0;
  for (const a of arrays) {
    result.set(a as Uint8Array, offset);
    offset += a.length;
  }
  return result;
}

/**
 * Helper: create a writable-signer account meta with the signer embedded.
 * Kit v2's signTransactionMessageWithSigners reads the `signer` field from
 * account metas to discover which signers to use. TypeScript's strict excess-
 * property checking prevents inline `{ signer }` in an `AccountMeta` union,
 * so we build it separately and let the structural type flow through.
 */
function signerMeta(signer: TransactionSigner) {
  return { address: signer.address, role: AccountRole.WRITABLE_SIGNER as const, signer };
}

function writable(addr: Address) {
  return { address: addr, role: AccountRole.WRITABLE as const };
}

function readonly(addr: Address) {
  return { address: addr, role: AccountRole.READONLY as const };
}

// =================================================================
// Permission Manager Instructions
// =================================================================

/**
 * Initialize the PermissionManager.
 *
 * Accounts:
 *   0. [signer, writable] Admin (payer, becomes admin)
 *   1. [writable]         PermissionConfig PDA
 *   2. [writable]         Admin's UserPermissions PDA
 *   3. []                 System program
 *
 * Data: disc(0) only
 */
export function initializePermissionManager(
  admin: TransactionSigner,
  permissionConfigPda: Address,
  adminPermsPda: Address
) {
  return {
    programAddress: PERMISSION_MANAGER_PROGRAM_ID,
    accounts: [
      signerMeta(admin),
      writable(permissionConfigPda),
      writable(adminPermsPda),
      readonly(SYSTEM_PROGRAM_ID),
    ],
    data: new Uint8Array([PM_DISC_INITIALIZE]) as ReadonlyUint8Array,
  };
}

/**
 * Grant a role to a target user.
 *
 * Accounts:
 *   0. [signer, writable] Caller (payer if target PDA needs creation)
 *   1. []                  PermissionConfig PDA
 *   2. [writable]          Target user's UserPermissions PDA
 *   3. []                  System program
 *   4. []                  Target user address
 *   5. []                  Caller's UserPermissions PDA
 *
 * Data: disc(1) + role_id(u8)
 */
export function grantRole(
  caller: TransactionSigner,
  permissionConfigPda: Address,
  targetUserPermsPda: Address,
  targetUser: Address,
  callerPermsPda: Address,
  roleId: number
) {
  return {
    programAddress: PERMISSION_MANAGER_PROGRAM_ID,
    accounts: [
      signerMeta(caller),
      readonly(permissionConfigPda),
      writable(targetUserPermsPda),
      readonly(SYSTEM_PROGRAM_ID),
      readonly(targetUser),
      readonly(callerPermsPda),
    ],
    data: new Uint8Array([PM_DISC_GRANT_ROLE, roleId]) as ReadonlyUint8Array,
  };
}

// =================================================================
// Spiko Token Instructions
// =================================================================

/**
 * Initialize a Spiko fund token (Token-2022 mint + TokenConfig).
 *
 * The mint account must be pre-allocated (use mintAccountSize() for the size, owned by Token-2022).
 * Extensions initialized: TransferHook, PermanentDelegate, MetadataPointer, TokenMetadata.
 *
 * Accounts (8):
 *   0. [signer, writable] Admin (payer)
 *   1. [writable]          TokenConfig PDA
 *   2. [writable]          Token-2022 Mint (pre-allocated)
 *   3. []                  MintAuthority PDA
 *   4. []                  PermissionManager program ID
 *   5. []                  Token-2022 program
 *   6. []                  System program
 *   7. []                  Transfer Hook program (spiko-transfer-hook)
 *
 * Data: disc(0) + decimals(u8) + name(4+N) + symbol(4+S) + uri(4+U)
 */
export function initializeToken(
  admin: TransactionSigner,
  tokenConfigPda: Address,
  mint: Address,
  mintAuthorityPda: Address,
  decimals: number,
  name: string,
  symbol: string,
  uri: string
) {
  const nameBytes = new TextEncoder().encode(name);
  const symbolBytes = new TextEncoder().encode(symbol);
  const uriBytes = new TextEncoder().encode(uri);

  const dataLen = 1 + 1 + 4 + nameBytes.length + 4 + symbolBytes.length + 4 + uriBytes.length;
  const data = new Uint8Array(dataLen);
  let offset = 0;

  // discriminator
  data[offset++] = ST_DISC_INITIALIZE;
  // decimals
  data[offset++] = decimals;
  // name (u32 LE length prefix + bytes)
  new DataView(data.buffer).setUint32(offset, nameBytes.length, true);
  offset += 4;
  data.set(nameBytes, offset);
  offset += nameBytes.length;
  // symbol
  new DataView(data.buffer).setUint32(offset, symbolBytes.length, true);
  offset += 4;
  data.set(symbolBytes, offset);
  offset += symbolBytes.length;
  // uri
  new DataView(data.buffer).setUint32(offset, uriBytes.length, true);
  offset += 4;
  data.set(uriBytes, offset);

  return {
    programAddress: SPIKO_TOKEN_PROGRAM_ID,
    accounts: [
      signerMeta(admin),
      writable(tokenConfigPda),
      writable(mint),
      readonly(mintAuthorityPda),
      readonly(PERMISSION_MANAGER_PROGRAM_ID),
      readonly(TOKEN_2022_PROGRAM_ID),
      readonly(SYSTEM_PROGRAM_ID),
      readonly(TRANSFER_HOOK_PROGRAM_ID),
    ],
    data: data as ReadonlyUint8Array,
  };
}

/**
 * Set the redemption contract on a TokenConfig.
 *
 * Accounts:
 *   0. [signer]   Admin caller
 *   1. [writable] TokenConfig PDA
 *   2. []         PermissionConfig PDA (from permission_manager)
 *
 * Data: disc(7) + redemption_contract(32 bytes)
 */
export function setRedemptionContract(
  admin: TransactionSigner,
  tokenConfigPda: Address,
  permissionConfigPda: Address,
  redemptionContract: Address
) {
  return {
    programAddress: SPIKO_TOKEN_PROGRAM_ID,
    accounts: [
      signerMeta(admin),
      writable(tokenConfigPda),
      readonly(permissionConfigPda),
    ],
    data: concat(
      new Uint8Array([ST_DISC_SET_REDEMPTION_CONTRACT]),
      encodeAddress(redemptionContract)
    ) as ReadonlyUint8Array,
  };
}

/**
 * Initialize the ExtraAccountMetaList PDA for a mint's transfer hook.
 *
 * Accounts (6):
 *   0. [signer, writable] Admin (payer)
 *   1. [writable]          ExtraAccountMetaList PDA
 *   2. []                  Token-2022 Mint
 *   3. []                  TokenConfig PDA
 *   4. []                  SpikoToken program
 *   5. []                  System program
 *
 * Data: disc(0) only
 */
export function initExtraAccountMetas(
  admin: TransactionSigner,
  extraAccountMetaListPda: Address,
  mint: Address,
  tokenConfigPda: Address
) {
  return {
    programAddress: TRANSFER_HOOK_PROGRAM_ID,
    accounts: [
      signerMeta(admin),
      writable(extraAccountMetaListPda),
      readonly(mint),
      readonly(tokenConfigPda),
      readonly(SPIKO_TOKEN_PROGRAM_ID),
      readonly(SYSTEM_PROGRAM_ID),
    ],
    data: new Uint8Array([TH_DISC_INIT_EXTRA_ACCOUNT_METAS]) as ReadonlyUint8Array,
  };
}

/**
 * Burn tokens from an account (calls SpikoToken directly).
 *
 * Accounts:
 *   0. [signer]   Caller (must have BURNER role)
 *   1. []         TokenConfig PDA
 *   2. [writable] Token-2022 Mint
 *   3. [writable] Source token account
 *   4. []         Mint authority PDA
 *   5. []         Caller's UserPermissions PDA
 *   6. []         Token-2022 program
 *
 * Data: disc(2) + amount(u64 LE)
 */
export function burnToken(
  caller: TransactionSigner,
  tokenConfigPda: Address,
  mint: Address,
  sourceTokenAccount: Address,
  mintAuthorityPda: Address,
  callerPermsPda: Address,
  amount: bigint
) {
  return {
    programAddress: SPIKO_TOKEN_PROGRAM_ID,
    accounts: [
      signerMeta(caller),
      readonly(tokenConfigPda),
      writable(mint),
      writable(sourceTokenAccount),
      readonly(mintAuthorityPda),
      readonly(callerPermsPda),
      readonly(TOKEN_2022_PROGRAM_ID),
    ],
    data: concat(
      new Uint8Array([ST_DISC_BURN]),
      encodeU64(amount)
    ) as ReadonlyUint8Array,
  };
}

/**
 * Transfer tokens via spiko-token program (Path B: program-mediated transfer).
 *
 * The program validates whitelist + pause, then CPIs into Token-2022
 * TransferChecked which triggers the transfer hook (double-validation).
 *
 * Accounts (12):
 *   0.  [signer]   Sender (owner of source token account)
 *   1.  []         TokenConfig PDA
 *   2.  [writable] Source token account (sender's ATA)
 *   3.  [writable] Destination token account (recipient's ATA)
 *   4.  []         Token-2022 Mint
 *   5.  []         Sender's UserPermissions PDA
 *   6.  []         Recipient's UserPermissions PDA
 *   7.  []         Token-2022 program
 *   8.  []         ExtraAccountMetaList PDA (transfer hook)
 *   9.  []         PermissionManager program
 *  10.  []         SpikoToken program (self, for hook resolution)
 *  11.  []         Transfer Hook program (spiko-transfer-hook)
 *
 * Data: disc(3) + amount(u64 LE)
 */
export function transferToken(
  sender: TransactionSigner,
  tokenConfigPda: Address,
  sourceAta: Address,
  destinationAta: Address,
  mint: Address,
  senderPermsPda: Address,
  recipientPermsPda: Address,
  extraAccountMetaListPda: Address,
  amount: bigint
) {
  return {
    programAddress: SPIKO_TOKEN_PROGRAM_ID,
    accounts: [
      signerMeta(sender),
      readonly(tokenConfigPda),
      writable(sourceAta),
      writable(destinationAta),
      readonly(mint),
      readonly(senderPermsPda),
      readonly(recipientPermsPda),
      readonly(TOKEN_2022_PROGRAM_ID),
      readonly(extraAccountMetaListPda),
      readonly(PERMISSION_MANAGER_PROGRAM_ID),
      readonly(SPIKO_TOKEN_PROGRAM_ID),
      readonly(TRANSFER_HOOK_PROGRAM_ID),
    ],
    data: concat(
      new Uint8Array([ST_DISC_TRANSFER]),
      encodeU64(amount)
    ) as ReadonlyUint8Array,
  };
}

/**
 * Build a direct Token-2022 TransferChecked instruction (Path A).
 *
 * This calls Token-2022 directly. The transfer hook will fire automatically
 * because the mint has a TransferHook extension configured.
 *
 * Standard TransferChecked accounts (4):
 *   0.  [writable] Source token account
 *   1.  []         Token-2022 Mint
 *   2.  [writable] Destination token account
 *   3.  [signer]   Authority (sender)
 *
 * Transfer Hook extra accounts (7):
 *   4.  []  ExtraAccountMetaList PDA
 *   5.  []  PermissionManager program
 *   6.  []  SpikoToken program
 *   7.  []  TokenConfig PDA
 *   8.  []  Sender's UserPermissions PDA
 *   9.  []  Recipient's UserPermissions PDA
 *  10.  []  Transfer Hook program
 *
 * Data: opcode(12) + amount(u64 LE) + decimals(u8)
 */
export function buildTransferChecked(
  sender: TransactionSigner,
  sourceAta: Address,
  destinationAta: Address,
  mint: Address,
  tokenConfigPda: Address,
  senderPermsPda: Address,
  recipientPermsPda: Address,
  extraAccountMetaListPda: Address,
  amount: bigint,
  decimals: number
) {
  // TransferChecked instruction data: [12, amount(8), decimals(1)]
  const data = new Uint8Array(10);
  data[0] = 12; // TransferChecked opcode
  const amountBytes = encodeU64(amount);
  data.set(amountBytes, 1);
  data[9] = decimals;

  return {
    programAddress: TOKEN_2022_PROGRAM_ID,
    accounts: [
      // Standard TransferChecked accounts
      writable(sourceAta),
      readonly(mint),
      writable(destinationAta),
      signerMeta(sender),
      // Transfer Hook extra accounts (must match ExtraAccountMetaList order)
      readonly(extraAccountMetaListPda),
      readonly(PERMISSION_MANAGER_PROGRAM_ID),
      readonly(SPIKO_TOKEN_PROGRAM_ID),
      readonly(tokenConfigPda),
      readonly(senderPermsPda),
      readonly(recipientPermsPda),
      readonly(TRANSFER_HOOK_PROGRAM_ID),
    ],
    data: data as ReadonlyUint8Array,
  };
}

// =================================================================
// Minter Instructions
// =================================================================

/**
 * Initialize the Minter.
 *
 * Accounts:
 *   0. [signer, writable] Admin (payer)
 *   1. [writable]          MinterConfig PDA
 *   2. []                  System program
 *
 * Data: disc(0) + max_delay(i64 LE) + permission_manager(32 bytes)
 */
export function initializeMinter(
  admin: TransactionSigner,
  minterConfigPda: Address,
  maxDelay: bigint,
  permissionManager: Address
) {
  return {
    programAddress: MINTER_PROGRAM_ID,
    accounts: [
      signerMeta(admin),
      writable(minterConfigPda),
      readonly(SYSTEM_PROGRAM_ID),
    ],
    data: concat(
      new Uint8Array([MT_DISC_INITIALIZE]),
      encodeI64(maxDelay),
      encodeAddress(permissionManager)
    ) as ReadonlyUint8Array,
  };
}

/**
 * Set daily mint limit for a token.
 *
 * Accounts:
 *   0. [signer, writable] Admin (payer if creating DailyLimit PDA)
 *   1. []                  MinterConfig PDA
 *   2. []                  PermissionConfig PDA (from permission_manager)
 *   3. [writable]          DailyLimit PDA
 *   4. []                  System program
 *
 * Data: disc(4) + token_mint(32 bytes) + limit(u64 LE)
 */
export function setDailyLimit(
  admin: TransactionSigner,
  minterConfigPda: Address,
  permissionConfigPda: Address,
  dailyLimitPda: Address,
  tokenMint: Address,
  limit: bigint
) {
  return {
    programAddress: MINTER_PROGRAM_ID,
    accounts: [
      signerMeta(admin),
      readonly(minterConfigPda),
      readonly(permissionConfigPda),
      writable(dailyLimitPda),
      readonly(SYSTEM_PROGRAM_ID),
    ],
    data: concat(
      new Uint8Array([MT_DISC_SET_DAILY_LIMIT]),
      encodeAddress(tokenMint),
      encodeU64(limit)
    ) as ReadonlyUint8Array,
  };
}

/**
 * Initiate a mint operation via the Minter program.
 * If within daily limit, executes immediately via CPI chain.
 * If over limit, creates a PENDING MintOperation PDA requiring approval.
 *
 * Accounts (14 total):
 *   0.  [signer]            Caller (must have ROLE_MINT_INITIATOR)
 *   1.  []                  MinterConfig PDA
 *   2.  [writable]          DailyLimit PDA for this token
 *   3.  [writable]          MintOperation PDA
 *   4.  []                  Caller's UserPermissions PDA
 *   5.  []                  Spiko Token program
 *   6.  []                  TokenConfig PDA (on spiko-token)
 *   7.  [writable]          Token-2022 Mint
 *   8.  [writable]          Recipient's token account
 *   9.  []                  Mint authority PDA (spiko-token)
 *  10.  []                  Minter's UserPermissions PDA (MinterConfig has ROLE_MINTER)
 *  11.  []                  Recipient's UserPermissions PDA (whitelist check)
 *  12.  []                  Token-2022 program
 *  13.  []                  System program
 *
 * Data: user(32B) + amount(u64 LE) + salt(u64 LE) = 48 bytes
 */
export function initiateMint(
  caller: TransactionSigner,
  minterConfigPda: Address,
  dailyLimitPda: Address,
  mintOperationPda: Address,
  callerPermsPda: Address,
  tokenConfigPda: Address,
  tokenMint: Address,
  recipientTokenAccount: Address,
  mintAuthorityPda: Address,
  minterUserPermsPda: Address,
  recipientPermsPda: Address,
  recipient: Address,
  amount: bigint,
  salt: bigint
) {
  return {
    programAddress: MINTER_PROGRAM_ID,
    accounts: [
      signerMeta(caller),
      writable(minterConfigPda),
      writable(dailyLimitPda),
      writable(mintOperationPda),
      readonly(callerPermsPda),
      readonly(SPIKO_TOKEN_PROGRAM_ID),
      readonly(tokenConfigPda),
      writable(tokenMint),
      writable(recipientTokenAccount),
      readonly(mintAuthorityPda),
      readonly(minterUserPermsPda),
      readonly(recipientPermsPda),
      readonly(TOKEN_2022_PROGRAM_ID),
      readonly(SYSTEM_PROGRAM_ID),
    ],
    data: concat(
      new Uint8Array([MT_DISC_INITIATE_MINT]),
      encodeAddress(recipient),
      encodeU64(amount),
      encodeU64(salt)
    ) as ReadonlyUint8Array,
  };
}

// =================================================================
// Redemption Instructions
// =================================================================

/**
 * Initialize the Redemption program.
 *
 * Accounts:
 *   0. [signer, writable] Admin (payer)
 *   1. [writable]          RedemptionConfig PDA
 *   2. []                  System program
 *
 * Data: disc(0) + permission_manager(32 bytes)
 */
export function initializeRedemption(
  admin: TransactionSigner,
  redemptionConfigPda: Address,
  permissionManager: Address
) {
  return {
    programAddress: REDEMPTION_PROGRAM_ID,
    accounts: [
      signerMeta(admin),
      writable(redemptionConfigPda),
      readonly(SYSTEM_PROGRAM_ID),
    ],
    data: concat(
      new Uint8Array([RD_DISC_INITIALIZE]),
      encodeAddress(permissionManager)
    ) as ReadonlyUint8Array,
  };
}

/**
 * Set minimum redemption amount for a token.
 *
 * Accounts:
 *   0. [signer, writable] Admin (payer if creating TokenMinimum PDA)
 *   1. []                  RedemptionConfig PDA
 *   2. []                  PermissionConfig PDA (from permission_manager)
 *   3. [writable]          TokenMinimum PDA
 *   4. []                  System program
 *
 * Data: disc(3) + token_mint(32 bytes) + minimum(u64 LE)
 */
export function setMinimum(
  admin: TransactionSigner,
  redemptionConfigPda: Address,
  permissionConfigPda: Address,
  tokenMinimumPda: Address,
  tokenMint: Address,
  minimum: bigint
) {
  return {
    programAddress: REDEMPTION_PROGRAM_ID,
    accounts: [
      signerMeta(admin),
      readonly(redemptionConfigPda),
      readonly(permissionConfigPda),
      writable(tokenMinimumPda),
      readonly(SYSTEM_PROGRAM_ID),
    ],
    data: concat(
      new Uint8Array([RD_DISC_SET_MINIMUM]),
      encodeAddress(tokenMint),
      encodeU64(minimum)
    ) as ReadonlyUint8Array,
  };
}

/**
 * Redeem tokens: transfer from user to vault, then CPI to Redemption::on_redeem.
 *
 * Accounts (17 total):
 *   0.  [signer]   User (token holder)
 *   1.  []         TokenConfig PDA
 *   2.  [writable] User's source token account
 *   3.  [writable] Vault token account (destination)
 *   4.  []         Token-2022 Mint
 *   5.  []         User's UserPermissions PDA (whitelist check)
 *   6.  []         Vault authority's UserPermissions PDA (whitelist check)
 *   7.  []         Token-2022 program
 *   8.  []         Redemption program
 *   9.  []         RedemptionConfig PDA
 *  10.  [writable] RedemptionOperation PDA (created by redemption)
 *  11.  []         TokenMinimum PDA
 *  12.  []         System program
 *  13.  []         ExtraAccountMetaList PDA (transfer hook)
 *  14.  []         PermissionManager program (transfer hook)
 *  15.  []         SpikoToken program (transfer hook — self-reference)
 *  16.  []         Transfer Hook program (spiko-transfer-hook)
 *
 * Data: disc(6) + amount(u64 LE) + salt(u64 LE) = 17 bytes
 */
export function redeemToken(
  user: TransactionSigner,
  tokenConfigPda: Address,
  userSourceAta: Address,
  vaultAta: Address,
  mint: Address,
  userPermsPda: Address,
  vaultAuthorityPermsPda: Address,
  redemptionConfigPda: Address,
  redemptionOpPda: Address,
  tokenMinimumPda: Address,
  extraAccountMetaListPda: Address,
  amount: bigint,
  salt: bigint
) {
  return {
    programAddress: SPIKO_TOKEN_PROGRAM_ID,
    accounts: [
      signerMeta(user),
      readonly(tokenConfigPda),
      writable(userSourceAta),
      writable(vaultAta),
      readonly(mint),
      readonly(userPermsPda),
      readonly(vaultAuthorityPermsPda),
      readonly(TOKEN_2022_PROGRAM_ID),
      readonly(REDEMPTION_PROGRAM_ID),
      readonly(redemptionConfigPda),
      writable(redemptionOpPda),
      readonly(tokenMinimumPda),
      readonly(SYSTEM_PROGRAM_ID),
      readonly(extraAccountMetaListPda),
      readonly(PERMISSION_MANAGER_PROGRAM_ID),
      readonly(SPIKO_TOKEN_PROGRAM_ID),
      readonly(TRANSFER_HOOK_PROGRAM_ID),
    ],
    data: concat(
      new Uint8Array([ST_DISC_REDEEM]),
      encodeU64(amount),
      encodeU64(salt)
    ) as ReadonlyUint8Array,
  };
}

/**
 * Execute a pending redemption. Burns tokens from the vault.
 *
 * Accounts (12 total):
 *   0.  [signer]   Operator (must have ROLE_REDEMPTION_EXECUTOR)
 *   1.  []         RedemptionConfig PDA
 *   2.  [writable] RedemptionOperation PDA
 *   3.  []         Operator's UserPermissions PDA
 *   4.  []         Spiko Token program
 *   5.  []         TokenConfig PDA (on spiko-token)
 *   6.  [writable] Token-2022 Mint
 *   7.  [writable] Vault token account (source for burn)
 *   8.  []         Mint authority PDA (spiko-token)
 *   9.  [writable] Vault authority PDA
 *  10.  []         Vault authority's UserPermissions PDA (ROLE_BURNER)
 *  11.  []         Token-2022 program
 *
 * Data: disc(1) + user(32) + amount(u64 LE) + salt(u64 LE) = 49 bytes
 */
export function executeRedemption(
  operator: TransactionSigner,
  redemptionConfigPda: Address,
  redemptionOpPda: Address,
  operatorPermsPda: Address,
  tokenConfigPda: Address,
  mint: Address,
  vaultTokenAccount: Address,
  mintAuthorityPda: Address,
  vaultAuthorityPda: Address,
  vaultAuthorityPermsPda: Address,
  user: Address,
  amount: bigint,
  salt: bigint
) {
  return {
    programAddress: REDEMPTION_PROGRAM_ID,
    accounts: [
      signerMeta(operator),
      readonly(redemptionConfigPda),
      writable(redemptionOpPda),
      readonly(operatorPermsPda),
      readonly(SPIKO_TOKEN_PROGRAM_ID),
      readonly(tokenConfigPda),
      writable(mint),
      writable(vaultTokenAccount),
      readonly(mintAuthorityPda),
      writable(vaultAuthorityPda),
      readonly(vaultAuthorityPermsPda),
      readonly(TOKEN_2022_PROGRAM_ID),
    ],
    data: concat(
      new Uint8Array([RD_DISC_EXECUTE]),
      encodeAddress(user),
      encodeU64(amount),
      encodeU64(salt)
    ) as ReadonlyUint8Array,
  };
}

/**
 * Cancel a redemption after the deadline has passed. Anyone can call.
 * Tokens are refunded from the vault back to the user.
 *
 * Accounts (15 total):
 *   0.  [signer]   Anyone (no permission check)
 *   1.  []         RedemptionConfig PDA
 *   2.  [writable] RedemptionOperation PDA
 *   3.  [writable] Vault token account (source for refund)
 *   4.  [writable] User's token account (destination for refund)
 *   5.  []         Vault authority PDA
 *   6.  []         Token-2022 Mint
 *   7.  []         Token-2022 program
 *   8.  []         ExtraAccountMetaList PDA (transfer hook)
 *   9.  []         PermissionManager program (transfer hook)
 *  10.  []         SpikoToken program (transfer hook)
 *  11.  []         TokenConfig PDA (transfer hook)
 *  12.  []         Vault authority's UserPermissions PDA (transfer hook)
 *  13.  []         User's UserPermissions PDA (transfer hook)
 *  14.  []         Transfer Hook program (spiko-transfer-hook)
 *
 * Data: disc(2) + user(32) + amount(u64 LE) + salt(u64 LE) = 49 bytes
 */
export function cancelRedemption(
  caller: TransactionSigner,
  redemptionConfigPda: Address,
  redemptionOpPda: Address,
  vaultTokenAccount: Address,
  userTokenAccount: Address,
  vaultAuthorityPda: Address,
  mint: Address,
  extraAccountMetaListPda: Address,
  tokenConfigPda: Address,
  vaultAuthorityPermsPda: Address,
  userPermsPda: Address,
  user: Address,
  amount: bigint,
  salt: bigint
) {
  return {
    programAddress: REDEMPTION_PROGRAM_ID,
    accounts: [
      signerMeta(caller),
      readonly(redemptionConfigPda),
      writable(redemptionOpPda),
      writable(vaultTokenAccount),
      writable(userTokenAccount),
      readonly(vaultAuthorityPda),
      readonly(mint),
      readonly(TOKEN_2022_PROGRAM_ID),
      readonly(extraAccountMetaListPda),
      readonly(PERMISSION_MANAGER_PROGRAM_ID),
      readonly(SPIKO_TOKEN_PROGRAM_ID),
      readonly(tokenConfigPda),
      readonly(vaultAuthorityPermsPda),
      readonly(userPermsPda),
      readonly(TRANSFER_HOOK_PROGRAM_ID),
    ],
    data: concat(
      new Uint8Array([RD_DISC_CANCEL]),
      encodeAddress(user),
      encodeU64(amount),
      encodeU64(salt)
    ) as ReadonlyUint8Array,
  };
}
