/**
 * Shared helpers for the E2E test.
 *
 * Uses Codama-generated PDA helpers and instruction builders for all 4 programs.
 * Hand-written helpers only for Token-2022 raw instructions
 * (CreateAccount, TransferChecked, InitializeMint2, extensions, etc.).
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
  generateKeyPairSigner,
} from "@solana/kit";

import fs from "fs";
import path from "path";

// ── Program addresses ────────────────────────────────────────
import { TRANSFER_HOOK_PROGRAM_ADDRESS } from "../../clients/ts/transfer-hook/src/generated/programs/index.js";
import { MINTER_PROGRAM_ADDRESS } from "../../clients/ts/minter/src/generated/programs/index.js";
import { REDEMPTION_PROGRAM_ADDRESS } from "../../clients/ts/redemption/src/generated/programs/index.js";
import { CUSTODIAL_GATEKEEPER_PROGRAM_ADDRESS } from "../../clients/ts/custodial-gatekeeper/src/generated/programs/index.js";

// ── Codama-generated PDA helpers ─────────────────────────────
import { findExtraAccountMetasPda, findHookConfigPda, findWhitelistStatePda } from "../../clients/ts/transfer-hook/src/generated/pdas/index.js";
import { findMinterConfigPda, findMintDailyLimitPda, findMintOperationPda } from "../../clients/ts/minter/src/generated/pdas/index.js";
import { findRedemptionConfigPda, findRedemptionRecordPda, findVaultAuthorityPda as findRedemptionVaultAuthorityPda } from "../../clients/ts/redemption/src/generated/pdas/index.js";
import { findGatekeeperConfigPda, findGatekeepOperationPda, findVaultAuthorityPda as findCgVaultAuthorityPda, findWithdrawalDailyLimitPda } from "../../clients/ts/custodial-gatekeeper/src/generated/pdas/index.js";

// ── Codama-generated instruction builders ────────────────────
// Transfer hook
export { getInitializeInstructionAsync as getThInitializeInstructionAsync } from "../../clients/ts/transfer-hook/src/generated/instructions/initialize.js";
export { getRegisterMintInstructionAsync } from "../../clients/ts/transfer-hook/src/generated/instructions/registerMint.js";
export { getWhitelistInstructionAsync } from "../../clients/ts/transfer-hook/src/generated/instructions/whitelist.js";
export { getAddGateInstructionAsync } from "../../clients/ts/transfer-hook/src/generated/instructions/addGate.js";
export { getUnwhitelistInstructionAsync } from "../../clients/ts/transfer-hook/src/generated/instructions/unwhitelist.js";
export { getRemoveGateInstructionAsync } from "../../clients/ts/transfer-hook/src/generated/instructions/removeGate.js";

// Minter
export { getInitializeInstructionAsync as getMtInitializeInstructionAsync } from "../../clients/ts/minter/src/generated/instructions/initialize.js";
export { getInitiateMintInstructionAsync } from "../../clients/ts/minter/src/generated/instructions/initiateMint.js";
export { getApproveMintInstructionAsync } from "../../clients/ts/minter/src/generated/instructions/approveMint.js";
export { getCancelMintInstructionAsync } from "../../clients/ts/minter/src/generated/instructions/cancelMint.js";
export { getSetDailyLimitInstructionAsync as getMtSetDailyLimitInstructionAsync } from "../../clients/ts/minter/src/generated/instructions/setDailyLimit.js";
export { getSetMintInitiatorInstructionAsync } from "../../clients/ts/minter/src/generated/instructions/setMintInitiator.js";

// Redemption
export { getInitializeInstructionAsync as getRdInitializeInstructionAsync } from "../../clients/ts/redemption/src/generated/instructions/initialize.js";
export { getRedeemInstructionAsync } from "../../clients/ts/redemption/src/generated/instructions/redeem.js";

// Custodial Gatekeeper
export { getInitializeInstructionAsync as getCgInitializeInstructionAsync } from "../../clients/ts/custodial-gatekeeper/src/generated/instructions/initialize.js";
export { getInitiateGatekeepInstructionAsync } from "../../clients/ts/custodial-gatekeeper/src/generated/instructions/initiateGatekeep.js";
export { getApproveGatekeepInstructionAsync } from "../../clients/ts/custodial-gatekeeper/src/generated/instructions/approveGatekeep.js";
export { getCancelGatekeepInstructionAsync } from "../../clients/ts/custodial-gatekeeper/src/generated/instructions/cancelGatekeep.js";
export { getSetDailyLimitInstructionAsync as getCgSetDailyLimitInstructionAsync } from "../../clients/ts/custodial-gatekeeper/src/generated/instructions/setDailyLimit.js";
export { getSetGatekeeperInitiatorInstructionAsync } from "../../clients/ts/custodial-gatekeeper/src/generated/instructions/setGatekeeperInitiator.js";

export {
  TRANSFER_HOOK_PROGRAM_ADDRESS,
  MINTER_PROGRAM_ADDRESS,
  REDEMPTION_PROGRAM_ADDRESS,
  CUSTODIAL_GATEKEEPER_PROGRAM_ADDRESS,
  findExtraAccountMetasPda,
  findHookConfigPda,
  findWhitelistStatePda,
  findMinterConfigPda,
  findMintDailyLimitPda,
  findMintOperationPda,
  findRedemptionConfigPda,
  findRedemptionRecordPda,
  findRedemptionVaultAuthorityPda,
  findGatekeeperConfigPda,
  findGatekeepOperationPda,
  findCgVaultAuthorityPda,
  findWithdrawalDailyLimitPda,
};

export { generateKeyPairSigner };

// ── Constants ────────────────────────────────────────────────

export const TOKEN_2022_PROGRAM_ID: Address =
  address("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

export const SYSTEM_PROGRAM_ID: Address =
  address("11111111111111111111111111111111");

export const ASSOCIATED_TOKEN_PROGRAM_ID: Address =
  address("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

// Role constants (matching transfer-hook program)
export const ROLE_WHITELISTED_GATE = 1;
export const ROLE_WHITELISTED = 2;
export const ROLE_UNWHITELISTED = 3;
export const ROLE_WHITELISTED_EXT = 4;
export const ROLE_UNWHITELISTED_EXT = 5;

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

export async function getTokenBalance(
  rpc: Rpc<SolanaRpcApi>,
  tokenAccount: Address
): Promise<bigint> {
  const { value } = await rpc
    .getAccountInfo(tokenAccount, { encoding: "base64" })
    .send();
  if (!value || !value.data) return 0n;
  const data = Buffer.from((value.data as any)[0], "base64");
  // Token account layout: mint[0..32], owner[32..64], amount[64..72]
  return data.readBigUInt64LE(64);
}

export async function getMintSupply(
  rpc: Rpc<SolanaRpcApi>,
  mint: Address
): Promise<bigint> {
  const { value } = await rpc
    .getAccountInfo(mint, { encoding: "base64" })
    .send();
  if (!value || !value.data) return 0n;
  const data = Buffer.from((value.data as any)[0], "base64");
  // Mint layout: COption<authority>[0..36], supply[36..44]
  return data.readBigUInt64LE(36);
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

/**
 * Send a transaction and expect it to fail. Returns true if it failed.
 */
export async function sendTxExpectFail(
  rpc: Rpc<SolanaRpcApi>,
  rpcSubscriptions: RpcSubscriptions<SolanaRpcSubscriptionsApi>,
  payer: KeyPairSigner,
  instructions: Parameters<typeof appendTransactionMessageInstructions>[0],
  label: string
): Promise<boolean> {
  try {
    await sendTx(rpc, rpcSubscriptions, payer, instructions, label);
    return false; // Did not fail
  } catch (e) {
    console.log(`  ${label}: FAILED as expected`);
    return true;
  }
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

// ── Token-2022 helpers ───────────────────────────────────────

const addressEnc = getAddressEncoder();

export async function getAssociatedTokenAddress(
  wallet: Address,
  mint: Address
): Promise<Address> {
  const [ata] = await getProgramDerivedAddress({
    programAddress: ASSOCIATED_TOKEN_PROGRAM_ID,
    seeds: [
      addressEnc.encode(wallet),
      addressEnc.encode(TOKEN_2022_PROGRAM_ID),
      addressEnc.encode(mint),
    ],
  });
  return ata;
}

/**
 * Build a CreateAssociatedTokenAccountIdempotent instruction for Token-2022.
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

/**
 * Build a TransferChecked instruction with transfer-hook extra accounts.
 *
 * Extra accounts order (matches ExtraAccountMetaList):
 *   1. extra_account_metas PDA
 *   2. hook_config PDA
 *   3. source_whitelist PDA (derived from source owner)
 *   4. dest_whitelist PDA (derived from dest owner)
 *   5. transfer_hook_program
 */
export function buildTransferChecked(
  sender: KeyPairSigner,
  sourceAta: Address,
  destinationAta: Address,
  mint: Address,
  extraAccountMetaListAddr: Address,
  hookConfigAddr: Address,
  sourceWhitelistAddr: Address,
  destWhitelistAddr: Address,
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
      // Transfer hook extra accounts
      { address: extraAccountMetaListAddr, role: AccountRole.READONLY as const },
      { address: hookConfigAddr, role: AccountRole.READONLY as const },
      { address: sourceWhitelistAddr, role: AccountRole.READONLY as const },
      { address: destWhitelistAddr, role: AccountRole.READONLY as const },
      { address: TRANSFER_HOOK_PROGRAM_ADDRESS as Address, role: AccountRole.READONLY as const },
    ],
    data: data as ReadonlyUint8Array,
  };
}

// ── Token-2022 mint initialization helpers ───────────────────

/**
 * Fixed extensions size: base mint + TransferHook + PermanentDelegate + MetadataPointer.
 */
export const MINT_FIXED_EXTENSIONS_SIZE = 338; // 166 + 68 + 36 + 68

export function getMintAccountSpace(
  name: string,
  symbol: string,
  uri: string
): number {
  const metadataDataLen =
    32 + 32 + (4 + name.length) + (4 + symbol.length) + (4 + uri.length) + 4;
  const TOKEN_METADATA = 4 + metadataDataLen;
  return MINT_FIXED_EXTENSIONS_SIZE + TOKEN_METADATA;
}

export function buildCreateAccountInstruction(
  payer: KeyPairSigner,
  newAccount: KeyPairSigner,
  lamports: bigint,
  space: number,
  owner: Address
) {
  const data = new Uint8Array(4 + 8 + 8 + 32);
  const view = new DataView(data.buffer);
  view.setUint32(0, 0, true);
  view.setBigUint64(4, lamports, true);
  view.setBigUint64(12, BigInt(space), true);
  data.set(addressEnc.encode(owner), 20);

  return {
    programAddress: SYSTEM_PROGRAM_ID,
    accounts: [
      { address: payer.address, role: AccountRole.WRITABLE_SIGNER as const, signer: payer },
      { address: newAccount.address, role: AccountRole.WRITABLE_SIGNER as const, signer: newAccount },
    ],
    data: data as ReadonlyUint8Array,
  };
}

export function buildInitializeTransferHook(
  mint: Address,
  authority: Address,
  hookProgramId: Address,
) {
  const data = new Uint8Array(2 + 32 + 32);
  data[0] = 36;
  data[1] = 0;
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

export function buildInitializePermanentDelegate(
  mint: Address,
  delegate: Address,
) {
  const data = new Uint8Array(1 + 32);
  data[0] = 35;
  data.set(addressEnc.encode(delegate), 1);

  return {
    programAddress: TOKEN_2022_PROGRAM_ID,
    accounts: [
      { address: mint, role: AccountRole.WRITABLE as const },
    ],
    data: data as ReadonlyUint8Array,
  };
}

export function buildInitializeMetadataPointer(
  mint: Address,
  authority: Address,
  metadataAddress: Address,
) {
  const data = new Uint8Array(2 + 32 + 32);
  data[0] = 39;
  data[1] = 0;
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

export function buildInitializeMint2(
  mint: Address,
  decimals: number,
  mintAuthority: Address,
) {
  const data = new Uint8Array(1 + 1 + 32 + 1);
  data[0] = 20;
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

  const disc = new Uint8Array([210, 225, 30, 162, 88, 184, 77, 141]);
  data.set(disc, 0);

  let offset = 8;
  view.setUint32(offset, nameBytes.length, true);
  offset += 4;
  data.set(nameBytes, offset);
  offset += nameBytes.length;

  view.setUint32(offset, symbolBytes.length, true);
  offset += 4;
  data.set(symbolBytes, offset);
  offset += symbolBytes.length;

  view.setUint32(offset, uriBytes.length, true);
  offset += 4;
  data.set(uriBytes, offset);

  return {
    programAddress: TOKEN_2022_PROGRAM_ID,
    accounts: [
      { address: mint, role: AccountRole.WRITABLE as const },
      { address: updateAuthority, role: AccountRole.READONLY as const },
      { address: mint, role: AccountRole.READONLY as const },
      { address: mintAuthority.address, role: AccountRole.READONLY_SIGNER as const, signer: mintAuthority },
    ],
    data: data as ReadonlyUint8Array,
  };
}

/**
 * Create a token account owned by a PDA (not an ATA).
 * Uses Token-2022 CreateAccount + InitializeAccount3 (opcode 18).
 */
export function buildCreateTokenAccount(
  payer: KeyPairSigner,
  tokenAccount: KeyPairSigner,
  lamports: bigint,
  mint: Address,
  owner: Address, // PDA address
) {
  const TOKEN_ACCOUNT_SIZE = 165;
  const createAccount = buildCreateAccountInstruction(
    payer,
    tokenAccount,
    lamports,
    TOKEN_ACCOUNT_SIZE,
    TOKEN_2022_PROGRAM_ID,
  );

  // InitializeAccount3: opcode(1) + owner(32)
  const initData = new Uint8Array(1 + 32);
  initData[0] = 18; // InitializeAccount3
  initData.set(addressEnc.encode(owner), 1);

  const initAccount = {
    programAddress: TOKEN_2022_PROGRAM_ID,
    accounts: [
      { address: tokenAccount.address, role: AccountRole.WRITABLE as const },
      { address: mint, role: AccountRole.READONLY as const },
    ],
    data: initData as ReadonlyUint8Array,
  };

  return [createAccount, initAccount];
}

/**
 * Get minimum rent for a given account size.
 */
export async function getMinimumRent(
  rpc: Rpc<SolanaRpcApi>,
  size: number
): Promise<bigint> {
  const rent = await rpc.getMinimumBalanceForRentExemption(BigInt(size)).send();
  return rent;
}

/**
 * Append transfer-hook extra accounts to an instruction (for CPI calls that trigger the hook).
 * Needed for custodial-gatekeeper and redemption instructions that do internal TransferChecked.
 */
export function appendHookAccounts(
  instruction: any,
  extraAccountMetasAddr: Address,
  hookConfigAddr: Address,
  sourceWhitelistAddr: Address,
  destWhitelistAddr: Address,
) {
  const hookAccounts = [
    { address: extraAccountMetasAddr, role: AccountRole.READONLY as const },
    { address: hookConfigAddr, role: AccountRole.READONLY as const },
    { address: sourceWhitelistAddr, role: AccountRole.READONLY as const },
    { address: destWhitelistAddr, role: AccountRole.READONLY as const },
    { address: TRANSFER_HOOK_PROGRAM_ADDRESS as Address, role: AccountRole.READONLY as const },
  ];
  return {
    ...instruction,
    accounts: [...instruction.accounts, ...hookAccounts],
  };
}

export function buildSetAuthority(
  account: Address,
  currentAuthority: KeyPairSigner,
  authorityType: number,
  newAuthority: Address | null,
) {
  const hasNew = newAuthority !== null;
  const data = new Uint8Array(1 + 1 + 1 + (hasNew ? 32 : 0));
  data[0] = 6; // SetAuthority opcode
  data[1] = authorityType;
  data[2] = hasNew ? 1 : 0; // COption tag
  if (hasNew) {
    data.set(addressEnc.encode(newAuthority), 3);
  }

  return {
    programAddress: TOKEN_2022_PROGRAM_ID,
    accounts: [
      { address: account, role: AccountRole.WRITABLE as const },
      { address: currentAuthority.address, role: AccountRole.READONLY_SIGNER as const, signer: currentAuthority },
    ],
    data: data as ReadonlyUint8Array,
  };
}
