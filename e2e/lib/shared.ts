/**
 * Shared helpers for the E2E test.
 *
 * Uses Codama-generated PDA helpers and program addresses.
 * Hand-written helpers only for Token-2022 raw instructions
 * (CreateAccount, TransferChecked, etc.).
 */

import {
  type Address,
  type KeyPairSigner,
  type Rpc,
  type SolanaRpcApi,
  type RpcSubscriptions,
  type SolanaRpcSubscriptionsApi,
  type ReadonlyUint8Array,
  address,
  AccountRole,
  createSolanaRpc,
  createSolanaRpcSubscriptions,
  createKeyPairSignerFromBytes,
  getAddressEncoder,
  getProgramDerivedAddress,
  pipe,
  createTransactionMessage,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  appendTransactionMessageInstructions,
  signTransactionMessageWithSigners,
  sendAndConfirmTransactionFactory,
  getSignatureFromTransaction,
} from "@solana/kit";

import fs from "fs";
import path from "path";

// ── Program addresses ────────────────────────────────────────
import { PERMISSION_MANAGER_PROGRAM_ADDRESS } from "../../clients/ts/permission-manager/src/generated/programs/index.js";
import { SPIKO_TOKEN_PROGRAM_ADDRESS } from "../../clients/ts/spiko-token/src/generated/programs/index.js";
import { MINTER_PROGRAM_ADDRESS } from "../../clients/ts/minter/src/generated/programs/index.js";
import { REDEMPTION_PROGRAM_ADDRESS } from "../../clients/ts/redemption/src/generated/programs/index.js";
import { SPIKO_TRANSFER_HOOK_PROGRAM_ADDRESS } from "../../clients/ts/spiko-transfer-hook/src/generated/programs/index.js";
import { CUSTODIAL_GATEKEEPER_PROGRAM_ADDRESS } from "../../clients/ts/custodial-gatekeeper/src/generated/programs/index.js";

// ── Codama-generated PDA helpers ─────────────────────────────
import { findConfigPda, findUserPermissionsPda } from "../../clients/ts/permission-manager/src/generated/pdas/index.js";
import { findTokenConfigPda, findMintAuthorityPda } from "../../clients/ts/spiko-token/src/generated/pdas/index.js";
import { findExtraAccountMetasPda, findHookConfigPda } from "../../clients/ts/spiko-transfer-hook/src/generated/pdas/index.js";
import { findMinterConfigPda, findMintDailyLimitPda, findMintOperationPda } from "../../clients/ts/minter/src/generated/pdas/index.js";
import { findRedemptionConfigPda, findRedemptionOperationPda, findVaultAuthorityPda as findRedemptionVaultAuthorityPda } from "../../clients/ts/redemption/src/generated/pdas/index.js";
import { findGatekeeperConfigPda, findVaultAuthorityPda as findCgVaultAuthorityPda, findWithdrawalDailyLimitPda, findWithdrawalOperationPda } from "../../clients/ts/custodial-gatekeeper/src/generated/pdas/index.js";

export {
  PERMISSION_MANAGER_PROGRAM_ADDRESS,
  SPIKO_TOKEN_PROGRAM_ADDRESS,
  MINTER_PROGRAM_ADDRESS,
  REDEMPTION_PROGRAM_ADDRESS,
  SPIKO_TRANSFER_HOOK_PROGRAM_ADDRESS,
  CUSTODIAL_GATEKEEPER_PROGRAM_ADDRESS,
  findConfigPda,
  findUserPermissionsPda,
  findTokenConfigPda,
  findMintAuthorityPda,
  findExtraAccountMetasPda,
  findHookConfigPda,
  findMinterConfigPda,
  findMintDailyLimitPda,
  findMintOperationPda,
  findRedemptionConfigPda,
  findRedemptionOperationPda,
  findRedemptionVaultAuthorityPda,
  findGatekeeperConfigPda,
  findCgVaultAuthorityPda,
  findWithdrawalDailyLimitPda,
  findWithdrawalOperationPda,
};

/**
 * Derive the Anchor event authority PDA for a given program.
 * Seeds: [b"__event_authority"]
 */
export async function findEventAuthorityPda(programAddress: Address): Promise<Address> {
  const [addr] = await getProgramDerivedAddress({
    programAddress,
    seeds: [new TextEncoder().encode("__event_authority")],
  });
  return addr;
}

export const TOKEN_2022_PROGRAM_ID: Address =
  address("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

export const SYSTEM_PROGRAM_ID: Address =
  address("11111111111111111111111111111111");

export const ASSOCIATED_TOKEN_PROGRAM_ID: Address =
  address("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

// ── Role constants (bitmask values matching permission_manager::constants) ──
export const ROLE_MINTER = 1;
export const ROLE_PAUSER = 2;
export const ROLE_BURNER = 4;
export const ROLE_WHITELISTER = 8;
export const ROLE_WHITELISTED = 16;
export const ROLE_REDEMPTION_EXECUTOR = 32;
export const ROLE_MINT_APPROVER = 64;
export const ROLE_MINT_INITIATOR = 128;
export const ROLE_WHITELISTED_EXT = 256;
export const ROLE_CUSTODIAL_GATEKEEPER_APPROVER = 512;

export const ROLE_NAMES: Record<number, string> = {
  [ROLE_MINTER]: "MINTER",
  [ROLE_PAUSER]: "PAUSER",
  [ROLE_BURNER]: "BURNER",
  [ROLE_WHITELISTER]: "WHITELISTER",
  [ROLE_WHITELISTED]: "WHITELISTED",
  [ROLE_REDEMPTION_EXECUTOR]: "REDEMPTION_EXECUTOR",
  [ROLE_MINT_APPROVER]: "MINT_APPROVER",
  [ROLE_MINT_INITIATOR]: "MINT_INITIATOR",
  [ROLE_WHITELISTED_EXT]: "WHITELISTED_EXT",
  [ROLE_CUSTODIAL_GATEKEEPER_APPROVER]: "CUSTODIAL_GATEKEEPER_APPROVER",
};

// ── Utility helpers ──────────────────────────────────────────
const addressEncoder = getAddressEncoder();

export function loadSolanaConfig(): { rpcUrl: string; keypairPath: string } {
  const configPath = path.join(
    process.env.HOME || "~",
    ".config/solana/cli/config.yml"
  );
  const content = fs.readFileSync(configPath, "utf-8");

  let rpcUrl = "https://api.devnet.solana.com";
  let keypairPath = path.join(
    process.env.HOME || "~",
    ".config/solana/id.json"
  );

  for (const line of content.split("\n")) {
    const trimmed = line.trim();
    if (trimmed.startsWith("json_rpc_url:")) {
      rpcUrl = trimmed.split("json_rpc_url:")[1].trim().replace(/"/g, "");
    }
    if (trimmed.startsWith("keypair_path:")) {
      keypairPath = trimmed.split("keypair_path:")[1].trim().replace(/"/g, "");
    }
  }

  return { rpcUrl, keypairPath };
}

function rpcUrlToWsUrl(rpcUrl: string): string {
  let wsUrl = rpcUrl.replace("https://", "wss://").replace("http://", "ws://");
  // Local validator uses port 8900 for WebSocket while RPC is on 8899
  if (wsUrl.includes("127.0.0.1:8899") || wsUrl.includes("localhost:8899")) {
    wsUrl = wsUrl.replace(":8899", ":8900");
  }
  return wsUrl;
}

export async function loadKeypair(filePath: string): Promise<KeyPairSigner> {
  const bytes = new Uint8Array(
    JSON.parse(fs.readFileSync(filePath, "utf-8"))
  );
  return createKeyPairSignerFromBytes(bytes);
}

export async function accountExists(
  rpc: Rpc<SolanaRpcApi>,
  addr: Address
): Promise<boolean> {
  const { value } = await rpc
    .getAccountInfo(addr, { encoding: "base64" })
    .send();
  return value !== null;
}

export async function sendTx(
  rpc: Rpc<SolanaRpcApi>,
  rpcSubscriptions: RpcSubscriptions<SolanaRpcSubscriptionsApi>,
  payer: KeyPairSigner,
  instructions: Parameters<typeof appendTransactionMessageInstructions>[0],
  label: string
): Promise<string> {
  const { value: latestBlockhash } = await rpc.getLatestBlockhash().send();

  const txMessage = pipe(
    createTransactionMessage({ version: 0 }),
    (tx) => setTransactionMessageFeePayerSigner(payer, tx),
    (tx) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, tx),
    (tx) => appendTransactionMessageInstructions(instructions, tx)
  );

  const signedTx = await signTransactionMessageWithSigners(txMessage);
  const sendAndConfirm = sendAndConfirmTransactionFactory({
    rpc,
    rpcSubscriptions,
  });
  await sendAndConfirm(signedTx, { commitment: "confirmed" });

  const sig = getSignatureFromTransaction(signedTx);
  console.log(`  ${label}: ${sig}`);
  return sig;
}

export async function setup() {
  const config = loadSolanaConfig();
  const rpc = createSolanaRpc(config.rpcUrl);
  const rpcSub = createSolanaRpcSubscriptions(rpcUrlToWsUrl(config.rpcUrl));
  const admin = await loadKeypair(config.keypairPath);

  console.log(`RPC:      ${config.rpcUrl}`);
  console.log(`Wallet:   ${admin.address}\n`);

  return { rpc, rpcSub, admin };
}

export async function getAssociatedTokenAddress(
  wallet: Address,
  mint: Address
): Promise<Address> {
  const [ata] = await getProgramDerivedAddress({
    programAddress: ASSOCIATED_TOKEN_PROGRAM_ID,
    seeds: [
      addressEncoder.encode(wallet),
      addressEncoder.encode(TOKEN_2022_PROGRAM_ID),
      addressEncoder.encode(mint),
    ],
  });
  return ata;
}

/**
 * Build a CreateAssociatedTokenAccountIdempotent instruction for Token-2022.
 * ATA program instruction index 1 = CreateIdempotent.
 */
export function createAssociatedTokenAccountIdempotent(
  payer: { address: Address; [key: string]: any },
  ata: Address,
  wallet: Address,
  mint: Address
) {
  return {
    programAddress: ASSOCIATED_TOKEN_PROGRAM_ID,
    accounts: [
      { address: payer.address, role: AccountRole.WRITABLE_SIGNER as const, signer: payer },
      { address: ata, role: AccountRole.WRITABLE as const },
      { address: wallet, role: AccountRole.READONLY as const },
      { address: mint, role: AccountRole.READONLY as const },
      { address: SYSTEM_PROGRAM_ID, role: AccountRole.READONLY as const },
      { address: TOKEN_2022_PROGRAM_ID, role: AccountRole.READONLY as const },
    ],
    data: new Uint8Array([1]) as ReadonlyUint8Array,
  };
}

export async function computeOperationId(
  user: Address,
  mint: Address,
  amount: bigint,
  salt: bigint
): Promise<Uint8Array> {
  const input = new Uint8Array(80);
  input.set(addressEncoder.encode(user), 0);
  input.set(addressEncoder.encode(mint), 32);

  const amountBuf = new ArrayBuffer(8);
  new DataView(amountBuf).setBigUint64(0, amount, true);
  input.set(new Uint8Array(amountBuf), 64);

  const saltBuf = new ArrayBuffer(8);
  new DataView(saltBuf).setBigUint64(0, salt, true);
  input.set(new Uint8Array(saltBuf), 72);

  const hash = await crypto.subtle.digest("SHA-256", input);
  return new Uint8Array(hash);
}

/**
 * Compute the required account space for a Token-2022 mint with
 * TransferHook + PermanentDelegate + MetadataPointer + inline metadata.
 *
 * Layout:
 *   BASE_MINT (82) + padding to multisig boundary (83) = 165
 *   + ACCOUNT_TYPE (1) = 166
 *   + each extension: TYPE_u16(2) + LENGTH_u16(2) + data
 *
 * Extensions:
 *   TransferHook:     2+2 + 64 (authority 32 + programId 32) = 68
 *   PermanentDelegate: 2+2 + 32 (delegate) = 36
 *   MetadataPointer:  2+2 + 64 (authority 32 + metadataAddress 32) = 68
 *   TokenMetadata:    2+2 + variable
 *     update_authority(32) + mint(32) + name(4+len) + symbol(4+len) + uri(4+len)
 *     + additional_metadata_len(4) = variable
 */
/**
 * Fixed extensions size: base mint + TransferHook + PermanentDelegate + MetadataPointer.
 * Token-2022 InitializeMint2 validates against this exact size.
 * Metadata is added later by TokenMetadataInitialize which reallocs the account.
 */
export const MINT_FIXED_EXTENSIONS_SIZE = 338; // 166 + 68 + 36 + 68

/**
 * Full account size including TokenMetadata extension (used for rent calculation).
 * Token-2022's TokenMetadataInitialize will realloc the account to this size.
 */
export function getMintAccountSpace(
  name: string,
  symbol: string,
  uri: string
): number {
  // TokenMetadata extension: type(2) + len(2) + data
  //   data = update_authority(32) + mint(32)
  //        + name_len(4) + name_bytes + symbol_len(4) + symbol_bytes + uri_len(4) + uri_bytes
  //        + additional_metadata_count(4)
  const metadataDataLen =
    32 + 32 + (4 + name.length) + (4 + symbol.length) + (4 + uri.length) + 4;
  const TOKEN_METADATA = 4 + metadataDataLen;

  return MINT_FIXED_EXTENSIONS_SIZE + TOKEN_METADATA;
}

/**
 * Build a SystemProgram::CreateAccount instruction to allocate the mint account.
 * The mint keypair must sign the transaction.
 */
export function buildCreateAccountInstruction(
  payer: KeyPairSigner,
  newAccount: KeyPairSigner,
  lamports: bigint,
  space: number,
  owner: Address
) {
  // SystemProgram CreateAccount instruction data:
  //   index(4 LE) = 0 + lamports(8 LE) + space(8 LE) + owner(32)
  const data = new Uint8Array(4 + 8 + 8 + 32);
  const view = new DataView(data.buffer);
  view.setUint32(0, 0, true); // CreateAccount instruction index
  view.setBigUint64(4, lamports, true);
  view.setBigUint64(12, BigInt(space), true);
  data.set(addressEncoder.encode(owner), 20);

  return {
    programAddress: SYSTEM_PROGRAM_ID,
    accounts: [
      { address: payer.address, role: AccountRole.WRITABLE_SIGNER as const, signer: payer },
      { address: newAccount.address, role: AccountRole.WRITABLE_SIGNER as const, signer: newAccount },
    ],
    data: data as ReadonlyUint8Array,
  };
}

/**
 * Build a raw Token-2022 TransferChecked with transfer hook extra accounts.
 * Hand-written since Codama only generates Spiko program clients.
 *
 * Transfer hook extra accounts order (matches ExtraAccountMetaList):
 *   1. extra_account_metas_list PDA (transfer-hook program)
 *   2. hook_config PDA (transfer-hook program)
 *   3. permission_manager_program
 *   4. permission_manager_config PDA
 *   5. source_permissions PDA (permission-manager)
 *   6. destination_permissions PDA (permission-manager)
 *   7. event_authority PDA (transfer-hook program)
 *   8. transfer_hook_program
 */
export function buildTransferChecked(
  sender: KeyPairSigner,
  sourceAta: Address,
  destinationAta: Address,
  mint: Address,
  hookConfigAddr: Address,
  senderPermsAddr: Address,
  recipientPermsAddr: Address,
  extraAccountMetaListAddr: Address,
  permissionManagerConfigAddr: Address,
  hookEventAuthorityAddr: Address,
  amount: bigint,
  decimals: number,
) {
  // Data: opcode(1) + amount(u64 LE) + decimals(u8) = 10 bytes
  const data = new Uint8Array(10);
  data[0] = 12; // TransferChecked opcode
  new DataView(data.buffer).setBigUint64(1, amount, true);
  data[9] = decimals;

  return {
    programAddress: TOKEN_2022_PROGRAM_ID,
    accounts: [
      { address: sourceAta, role: AccountRole.WRITABLE as const },
      { address: mint, role: AccountRole.READONLY as const },
      { address: destinationAta, role: AccountRole.WRITABLE as const },
      { address: sender.address, role: AccountRole.WRITABLE_SIGNER as const, signer: sender },
      // Transfer hook extra accounts (order must match ExtraAccountMetaList)
      { address: extraAccountMetaListAddr, role: AccountRole.READONLY as const },
      { address: hookConfigAddr, role: AccountRole.READONLY as const },
      { address: PERMISSION_MANAGER_PROGRAM_ADDRESS as Address, role: AccountRole.READONLY as const },
      { address: permissionManagerConfigAddr, role: AccountRole.READONLY as const },
      { address: senderPermsAddr, role: AccountRole.READONLY as const },
      { address: recipientPermsAddr, role: AccountRole.READONLY as const },
      { address: hookEventAuthorityAddr, role: AccountRole.READONLY as const },
      { address: SPIKO_TRANSFER_HOOK_PROGRAM_ADDRESS as Address, role: AccountRole.READONLY as const },
    ],
    data: data as ReadonlyUint8Array,
  };
}

// ── Token-2022 mint initialization helpers ───────────────────

const addressEnc = getAddressEncoder();

/**
 * Token-2022 InitializeTransferHook instruction.
 * Must be called BEFORE InitializeMint2.
 * Opcode: 36 (u8), sub-opcode: 0 (u8), authority(32), programId(32)
 */
export function buildInitializeTransferHook(
  mint: Address,
  authority: Address,
  hookProgramId: Address,
) {
  const data = new Uint8Array(2 + 32 + 32);
  data[0] = 36; // TransferHookExtension instruction
  data[1] = 0;  // InitializeTransferHook sub-instruction
  data.set(addressEnc.encode(authority), 2);
  data.set(addressEnc.encode(hookProgramId), 34);

  return {
    programAddress: TOKEN_2022_PROGRAM_ID,
    accounts: [
      { address: mint, role: AccountRole.WRITABLE as const },
    ],
    data: data as ReadonlyUint8Array,
  };
}

/**
 * Token-2022 InitializePermanentDelegate instruction.
 * Must be called BEFORE InitializeMint2.
 * Opcode: 35 (u8), delegate(32)
 */
export function buildInitializePermanentDelegate(
  mint: Address,
  delegate: Address,
) {
  const data = new Uint8Array(1 + 32);
  data[0] = 35; // InitializePermanentDelegate
  data.set(addressEnc.encode(delegate), 1);

  return {
    programAddress: TOKEN_2022_PROGRAM_ID,
    accounts: [
      { address: mint, role: AccountRole.WRITABLE as const },
    ],
    data: data as ReadonlyUint8Array,
  };
}

/**
 * Token-2022 InitializeMetadataPointer instruction.
 * Must be called BEFORE InitializeMint2.
 * Opcode: 39 (u8), sub-opcode: 0 (u8), authority(32), metadataAddress(32)
 */
export function buildInitializeMetadataPointer(
  mint: Address,
  authority: Address,
  metadataAddress: Address,
) {
  const data = new Uint8Array(2 + 32 + 32);
  data[0] = 39; // MetadataPointerExtension instruction
  data[1] = 0;  // InitializeMetadataPointer sub-instruction
  data.set(addressEnc.encode(authority), 2);
  data.set(addressEnc.encode(metadataAddress), 34);

  return {
    programAddress: TOKEN_2022_PROGRAM_ID,
    accounts: [
      { address: mint, role: AccountRole.WRITABLE as const },
    ],
    data: data as ReadonlyUint8Array,
  };
}

/**
 * Token-2022 InitializeMint2 instruction (no freeze authority).
 * Opcode: 20 (u8), decimals(u8), mintAuthority(32), freezeOption(u8=0)
 */
export function buildInitializeMint2(
  mint: Address,
  decimals: number,
  mintAuthority: Address,
) {
  const data = new Uint8Array(1 + 1 + 32 + 1);
  data[0] = 20; // InitializeMint2
  data[1] = decimals;
  data.set(addressEnc.encode(mintAuthority), 2);
  data[34] = 0; // no freeze authority

  return {
    programAddress: TOKEN_2022_PROGRAM_ID,
    accounts: [
      { address: mint, role: AccountRole.WRITABLE as const },
    ],
    data: data as ReadonlyUint8Array,
  };
}

/**
 * spl_token_metadata_interface Initialize instruction.
 * Emitted as a Token-2022 instruction against the mint with metadata pointer.
 *
 * Discriminator: first 8 bytes of hash("spl_token_metadata_interface:initialize_account")
 *   = [210, 225, 30, 162, 88, 184, 118, 21]
 * Then: name(borsh string: u32_le + bytes), symbol(borsh string), uri(borsh string)
 */
export function buildTokenMetadataInitialize(
  mint: Address,
  updateAuthority: Address,
  mintAuthority: KeyPairSigner,
  name: string,
  symbol: string,
  uri: string,
) {
  const nameBytes = new TextEncoder().encode(name);
  const symbolBytes = new TextEncoder().encode(symbol);
  const uriBytes = new TextEncoder().encode(uri);

  const dataLen = 8 + 4 + nameBytes.length + 4 + symbolBytes.length + 4 + uriBytes.length;
  const data = new Uint8Array(dataLen);
  const view = new DataView(data.buffer);

  // Discriminator: hash("spl_token_metadata_interface:initialize_account")[0..8]
  const disc = new Uint8Array([210, 225, 30, 162, 88, 184, 77, 141]);
  data.set(disc, 0);

  let offset = 8;

  // name (borsh string)
  view.setUint32(offset, nameBytes.length, true);
  offset += 4;
  data.set(nameBytes, offset);
  offset += nameBytes.length;

  // symbol (borsh string)
  view.setUint32(offset, symbolBytes.length, true);
  offset += 4;
  data.set(symbolBytes, offset);
  offset += symbolBytes.length;

  // uri (borsh string)
  view.setUint32(offset, uriBytes.length, true);
  offset += 4;
  data.set(uriBytes, offset);

  return {
    programAddress: TOKEN_2022_PROGRAM_ID,
    accounts: [
      { address: mint, role: AccountRole.WRITABLE as const },
      { address: updateAuthority, role: AccountRole.READONLY as const },
      { address: mint, role: AccountRole.READONLY as const }, // mint (same as metadata account since pointer points to self)
      { address: mintAuthority.address, role: AccountRole.READONLY_SIGNER as const, signer: mintAuthority },
    ],
    data: data as ReadonlyUint8Array,
  };
}
