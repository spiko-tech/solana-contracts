/**
 * Shared helpers for script_v2.
 *
 * Combines constants, PDA derivation, ATA helpers, RPC setup,
 * and the computeOperationId utility.
 *
 * Program addresses come from Codama-generated clients;
 * PDA seeds and role constants are defined here since Codama
 * did not generate PDA helpers.
 */

import fs from "fs";
import path from "path";

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

// Codama-generated program addresses
import { PERMISSION_MANAGER_PROGRAM_ADDRESS } from "../../clients/ts/permission-manager/programs/index.js";
import { SPIKO_TOKEN_PROGRAM_ADDRESS } from "../../clients/ts/spiko-token/programs/index.js";
import { MINTER_PROGRAM_ADDRESS } from "../../clients/ts/minter/programs/index.js";
import { REDEMPTION_PROGRAM_ADDRESS } from "../../clients/ts/redemption/programs/index.js";
import { SPIKO_TRANSFER_HOOK_PROGRAM_ADDRESS } from "../../clients/ts/spiko-transfer-hook/programs/index.js";

// Re-export for convenience
export {
  PERMISSION_MANAGER_PROGRAM_ADDRESS,
  SPIKO_TOKEN_PROGRAM_ADDRESS,
  MINTER_PROGRAM_ADDRESS,
  REDEMPTION_PROGRAM_ADDRESS,
  SPIKO_TRANSFER_HOOK_PROGRAM_ADDRESS,
};

// =================================================================
// Well-known program addresses
// =================================================================

export const TOKEN_2022_PROGRAM_ID: Address =
  address("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

export const SYSTEM_PROGRAM_ID: Address =
  address("11111111111111111111111111111111");

export const ASSOCIATED_TOKEN_PROGRAM_ID: Address =
  address("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

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
// PDA Seeds
// =================================================================

const enc = new TextEncoder();

const PERMISSION_CONFIG_SEED = enc.encode("permission_config");
const USER_PERMISSION_SEED = enc.encode("user_perm");
const TOKEN_CONFIG_SEED = enc.encode("token_config");
const MINT_AUTHORITY_SEED = enc.encode("mint_authority");
const EXTRA_ACCOUNT_METAS_SEED = enc.encode("extra-account-metas");
const MINTER_CONFIG_SEED = enc.encode("minter_config");
const DAILY_LIMIT_SEED = enc.encode("daily_limit");
const MINT_OPERATION_SEED = enc.encode("mint_op");
const REDEMPTION_CONFIG_SEED = enc.encode("redemption_config");
const TOKEN_MINIMUM_SEED = enc.encode("minimum");
const VAULT_SEED = enc.encode("vault");
const REDEMPTION_OPERATION_SEED = enc.encode("redemption_op");

// =================================================================
// PDA Derivation
// =================================================================

const addressEncoder = getAddressEncoder();

// ── Permission Manager ──

export async function permissionConfigPda(): Promise<readonly [Address, number]> {
  return getProgramDerivedAddress({
    programAddress: PERMISSION_MANAGER_PROGRAM_ADDRESS,
    seeds: [PERMISSION_CONFIG_SEED],
  });
}

export async function userPermissionsPda(user: Address): Promise<readonly [Address, number]> {
  return getProgramDerivedAddress({
    programAddress: PERMISSION_MANAGER_PROGRAM_ADDRESS,
    seeds: [USER_PERMISSION_SEED, addressEncoder.encode(user)],
  });
}

// ── Spiko Token ──

export async function tokenConfigPda(mint: Address): Promise<readonly [Address, number]> {
  return getProgramDerivedAddress({
    programAddress: SPIKO_TOKEN_PROGRAM_ADDRESS,
    seeds: [TOKEN_CONFIG_SEED, addressEncoder.encode(mint)],
  });
}

export async function mintAuthorityPda(mint: Address): Promise<readonly [Address, number]> {
  return getProgramDerivedAddress({
    programAddress: SPIKO_TOKEN_PROGRAM_ADDRESS,
    seeds: [MINT_AUTHORITY_SEED, addressEncoder.encode(mint)],
  });
}

// ── Transfer Hook ──

export async function extraAccountMetaListPda(mint: Address): Promise<readonly [Address, number]> {
  return getProgramDerivedAddress({
    programAddress: SPIKO_TRANSFER_HOOK_PROGRAM_ADDRESS,
    seeds: [EXTRA_ACCOUNT_METAS_SEED, addressEncoder.encode(mint)],
  });
}

// ── Minter ──

export async function minterConfigPda(): Promise<readonly [Address, number]> {
  return getProgramDerivedAddress({
    programAddress: MINTER_PROGRAM_ADDRESS,
    seeds: [MINTER_CONFIG_SEED],
  });
}

export async function dailyLimitPda(mint: Address): Promise<readonly [Address, number]> {
  return getProgramDerivedAddress({
    programAddress: MINTER_PROGRAM_ADDRESS,
    seeds: [DAILY_LIMIT_SEED, addressEncoder.encode(mint)],
  });
}

export async function mintOperationPda(operationId: Uint8Array): Promise<readonly [Address, number]> {
  return getProgramDerivedAddress({
    programAddress: MINTER_PROGRAM_ADDRESS,
    seeds: [MINT_OPERATION_SEED, operationId],
  });
}

// ── Redemption ──

export async function redemptionConfigPda(): Promise<readonly [Address, number]> {
  return getProgramDerivedAddress({
    programAddress: REDEMPTION_PROGRAM_ADDRESS,
    seeds: [REDEMPTION_CONFIG_SEED],
  });
}

export async function tokenMinimumPda(mint: Address): Promise<readonly [Address, number]> {
  return getProgramDerivedAddress({
    programAddress: REDEMPTION_PROGRAM_ADDRESS,
    seeds: [TOKEN_MINIMUM_SEED, addressEncoder.encode(mint)],
  });
}

export async function vaultAuthorityPda(): Promise<readonly [Address, number]> {
  return getProgramDerivedAddress({
    programAddress: REDEMPTION_PROGRAM_ADDRESS,
    seeds: [VAULT_SEED],
  });
}

export async function redemptionOperationPda(operationId: Uint8Array): Promise<readonly [Address, number]> {
  return getProgramDerivedAddress({
    programAddress: REDEMPTION_PROGRAM_ADDRESS,
    seeds: [REDEMPTION_OPERATION_SEED, operationId],
  });
}

// =================================================================
// Solana CLI config loading
// =================================================================

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
  return rpcUrl.replace("https://", "wss://").replace("http://", "ws://");
}

// =================================================================
// Keypair + RPC helpers
// =================================================================

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

// =================================================================
// Setup — loads config, creates RPC, loads admin keypair
// =================================================================

export async function setup() {
  const config = loadSolanaConfig();
  const rpc = createSolanaRpc(config.rpcUrl);
  const rpcSub = createSolanaRpcSubscriptions(rpcUrlToWsUrl(config.rpcUrl));
  const admin = await loadKeypair(config.keypairPath);

  console.log(`RPC:      ${config.rpcUrl}`);
  console.log(`Wallet:   ${admin.address}\n`);

  return { rpc, rpcSub, admin };
}

// =================================================================
// ATA (Associated Token Account) helpers
// =================================================================

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

// =================================================================
// Mint address helpers
// =================================================================

const __dirname = path.dirname(new URL(import.meta.url).pathname);
const KEYS_DIR = path.join(__dirname, "..", "..", "scripts", "keys");

/**
 * Resolve a token name (eutbl/ustbl) to its mint address.
 * Reads from the keypair file in scripts/keys/.
 */
export async function resolveMintAddress(tokenName: string): Promise<Address> {
  const name = tokenName.toLowerCase();
  if (name !== "eutbl" && name !== "ustbl") {
    throw new Error(`Unknown token: ${tokenName}. Use 'eutbl' or 'ustbl'.`);
  }
  const keyPath = path.join(KEYS_DIR, `${name}-mint.json`);
  if (!fs.existsSync(keyPath)) {
    throw new Error(`Mint keypair not found at ${keyPath}. Run deploy.ts first.`);
  }
  const signer = await loadKeypair(keyPath);
  return signer.address;
}

// =================================================================
// Mint account helpers
// =================================================================

/**
 * Read the decimals byte from an on-chain Token-2022 mint account.
 * In the SPL Token / Token-2022 mint layout, decimals is at byte offset 44.
 */
export async function readMintDecimals(
  rpc: Rpc<SolanaRpcApi>,
  mint: Address
): Promise<number> {
  const { value } = await rpc
    .getAccountInfo(mint, { encoding: "base64" })
    .send();
  if (!value) throw new Error(`Mint account ${mint} not found on-chain`);
  const data = Buffer.from(
    (value.data as unknown as [string, string])[0],
    "base64"
  );
  if (data.length < 45) {
    throw new Error(`Mint account ${mint} data too short (${data.length} bytes)`);
  }
  return data[44];
}

// =================================================================
// SHA-256 helper for operation_id
// =================================================================

/**
 * Compute operation_id = SHA256(user(32) || mint(32) || amount_le(8) || salt_le(8))
 * Uses Web Crypto API (available in Node.js 18+).
 */
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

// =================================================================
// buildTransferChecked (Path A: direct Token-2022 TransferChecked)
//
// This is NOT a Spiko program instruction — it's a raw Token-2022
// TransferChecked with extra accounts for the transfer hook.
// Stays hand-written since Codama only generates Spiko program clients.
// =================================================================

export function buildTransferChecked(
  sender: KeyPairSigner,
  sourceAta: Address,
  destinationAta: Address,
  mint: Address,
  tokenConfigAddr: Address,
  senderPermsAddr: Address,
  recipientPermsAddr: Address,
  extraAccountMetaListAddr: Address,
  amount: bigint,
  decimals: number
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
      { address: PERMISSION_MANAGER_PROGRAM_ADDRESS as Address, role: AccountRole.READONLY as const },
      { address: SPIKO_TOKEN_PROGRAM_ADDRESS as Address, role: AccountRole.READONLY as const },
      { address: tokenConfigAddr, role: AccountRole.READONLY as const },
      { address: senderPermsAddr, role: AccountRole.READONLY as const },
      { address: recipientPermsAddr, role: AccountRole.READONLY as const },
      { address: SPIKO_TRANSFER_HOOK_PROGRAM_ADDRESS as Address, role: AccountRole.READONLY as const },
    ],
    data: data as ReadonlyUint8Array,
  };
}
