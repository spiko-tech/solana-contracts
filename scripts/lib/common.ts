/**
 * Shared helpers for deployment scripts.
 * Independent from e2e/ — self-contained.
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

// ── Program addresses ────────────────────────────────────────
import { TRANSFER_HOOK_PROGRAM_ADDRESS } from "../../clients/ts/transfer-hook/src/generated/programs/index.js";
import { MINTER_PROGRAM_ADDRESS } from "../../clients/ts/minter/src/generated/programs/index.js";
import { REDEMPTION_PROGRAM_ADDRESS } from "../../clients/ts/redemption/src/generated/programs/index.js";
import { CUSTODIAL_GATEKEEPER_PROGRAM_ADDRESS } from "../../clients/ts/custodial-gatekeeper/src/generated/programs/index.js";

// ── Codama-generated PDA helpers ─────────────────────────────
import { findExtraAccountMetasPda, findHookConfigPda, findWhitelistStatePda } from "../../clients/ts/transfer-hook/src/generated/pdas/index.js";
import { findMinterConfigPda, findMintDailyLimitPda } from "../../clients/ts/minter/src/generated/pdas/index.js";
import { findRedemptionConfigPda, findVaultAuthorityPda as findRedemptionVaultAuthorityPda } from "../../clients/ts/redemption/src/generated/pdas/index.js";
import { findGatekeeperConfigPda, findVaultAuthorityPda as findCgVaultAuthorityPda, findWithdrawalDailyLimitPda } from "../../clients/ts/custodial-gatekeeper/src/generated/pdas/index.js";

// ── Codama-generated instruction builders ────────────────────
// Transfer hook
export { getInitializeInstructionAsync as getThInitializeInstructionAsync } from "../../clients/ts/transfer-hook/src/generated/instructions/initialize.js";
export { getRegisterMintInstructionAsync } from "../../clients/ts/transfer-hook/src/generated/instructions/registerMint.js";
export { getWhitelistInstructionAsync } from "../../clients/ts/transfer-hook/src/generated/instructions/whitelist.js";
export { getAddGateInstructionAsync } from "../../clients/ts/transfer-hook/src/generated/instructions/addGate.js";

// Minter
export { getInitializeInstructionAsync as getMtInitializeInstructionAsync } from "../../clients/ts/minter/src/generated/instructions/initialize.js";
export { getSetDailyLimitInstructionAsync as getMtSetDailyLimitInstructionAsync } from "../../clients/ts/minter/src/generated/instructions/setDailyLimit.js";

// Redemption
export { getInitializeInstructionAsync as getRdInitializeInstructionAsync } from "../../clients/ts/redemption/src/generated/instructions/initialize.js";

// Custodial Gatekeeper
export { getInitializeInstructionAsync as getCgInitializeInstructionAsync } from "../../clients/ts/custodial-gatekeeper/src/generated/instructions/initialize.js";
export { getSetDailyLimitInstructionAsync as getCgSetDailyLimitInstructionAsync } from "../../clients/ts/custodial-gatekeeper/src/generated/instructions/setDailyLimit.js";

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
  findRedemptionConfigPda,
  findRedemptionVaultAuthorityPda,
  findGatekeeperConfigPda,
  findCgVaultAuthorityPda,
  findWithdrawalDailyLimitPda,
  generateKeyPairSigner,
};

// ── Constants ────────────────────────────────────────────────

export const TOKEN_2022_PROGRAM_ID: Address =
  address("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

export const SYSTEM_PROGRAM_ID: Address =
  address("11111111111111111111111111111111");

export const ASSOCIATED_TOKEN_PROGRAM_ID: Address =
  address("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

// ── RPC helpers ──────────────────────────────────────────────

const CLUSTER_URLS: Record<string, string> = {
  localnet: "http://127.0.0.1:8899",
  devnet: "https://api.devnet.solana.com",
  "mainnet-beta": "https://api.mainnet-beta.solana.com",
};

function rpcUrlToWsUrl(rpcUrl: string): string {
  let wsUrl = rpcUrl.replace("https://", "wss://").replace("http://", "ws://");
  if (wsUrl.includes("127.0.0.1:8899") || wsUrl.includes("localhost:8899")) {
    wsUrl = wsUrl.replace(":8899", ":8900");
  }
  return wsUrl;
}

export function getRpc(cluster: string): {
  rpc: Rpc<SolanaRpcApi>;
  rpcSub: RpcSubscriptions<SolanaRpcSubscriptionsApi>;
} {
  const url = CLUSTER_URLS[cluster];
  if (!url) throw new Error(`Unknown cluster: ${cluster}`);
  return {
    rpc: createSolanaRpc(url),
    rpcSub: createSolanaRpcSubscriptions(rpcUrlToWsUrl(url)),
  };
}

export async function loadKeypair(filePath: string): Promise<KeyPairSigner> {
  const bytes = new Uint8Array(JSON.parse(fs.readFileSync(filePath, "utf-8")));
  return createKeyPairSignerFromBytes(bytes);
}

export async function accountExists(
  rpc: Rpc<SolanaRpcApi>,
  addr: Address
): Promise<boolean> {
  const { value } = await rpc.getAccountInfo(addr, { encoding: "base64" }).send();
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
  console.log(`  ✓ ${label}: ${sig}`);
  return sig;
}

export async function getMinimumRent(
  rpc: Rpc<SolanaRpcApi>,
  size: number
): Promise<bigint> {
  return await rpc.getMinimumBalanceForRentExemption(BigInt(size)).send();
}

// ── Token-2022 mint helpers ──────────────────────────────────

const addressEnc = getAddressEncoder();

export const MINT_FIXED_EXTENSIONS_SIZE = 338; // base mint + TransferHook + PermanentDelegate + MetadataPointer

export function getMintAccountSpace(name: string, symbol: string, uri: string): number {
  const metadataDataLen = 32 + 32 + (4 + name.length) + (4 + symbol.length) + (4 + uri.length) + 4;
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
  hookProgramId: Address
) {
  const data = new Uint8Array(2 + 32 + 32);
  data[0] = 36;
  data[1] = 0;
  data.set(addressEnc.encode(authority), 2);
  data.set(addressEnc.encode(hookProgramId), 34);

  return {
    programAddress: TOKEN_2022_PROGRAM_ID,
    accounts: [{ address: mint, role: AccountRole.WRITABLE as const }],
    data: data as ReadonlyUint8Array,
  };
}

export function buildInitializePermanentDelegate(mint: Address, delegate: Address) {
  const data = new Uint8Array(1 + 32);
  data[0] = 35;
  data.set(addressEnc.encode(delegate), 1);

  return {
    programAddress: TOKEN_2022_PROGRAM_ID,
    accounts: [{ address: mint, role: AccountRole.WRITABLE as const }],
    data: data as ReadonlyUint8Array,
  };
}

export function buildInitializeMetadataPointer(
  mint: Address,
  authority: Address,
  metadataAddress: Address
) {
  const data = new Uint8Array(2 + 32 + 32);
  data[0] = 39;
  data[1] = 0;
  data.set(addressEnc.encode(authority), 2);
  data.set(addressEnc.encode(metadataAddress), 34);

  return {
    programAddress: TOKEN_2022_PROGRAM_ID,
    accounts: [{ address: mint, role: AccountRole.WRITABLE as const }],
    data: data as ReadonlyUint8Array,
  };
}

export function buildInitializeMint2(
  mint: Address,
  decimals: number,
  mintAuthority: Address
) {
  const data = new Uint8Array(1 + 1 + 32 + 1);
  data[0] = 20;
  data[1] = decimals;
  data.set(addressEnc.encode(mintAuthority), 2);
  data[34] = 0; // no freeze authority

  return {
    programAddress: TOKEN_2022_PROGRAM_ID,
    accounts: [{ address: mint, role: AccountRole.WRITABLE as const }],
    data: data as ReadonlyUint8Array,
  };
}

export function buildTokenMetadataInitialize(
  mint: Address,
  updateAuthority: Address,
  mintAuthority: KeyPairSigner,
  name: string,
  symbol: string,
  uri: string
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

export function buildSetAuthority(
  account: Address,
  currentAuthority: KeyPairSigner,
  authorityType: number,
  newAuthority: Address | null
) {
  const hasNew = newAuthority !== null;
  const data = new Uint8Array(1 + 1 + 1 + (hasNew ? 32 : 0));
  data[0] = 6; // SetAuthority opcode
  data[1] = authorityType;
  data[2] = hasNew ? 1 : 0;
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
