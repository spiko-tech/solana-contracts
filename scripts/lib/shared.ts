/**
 * Shared helpers for all operational scripts.
 * Extracted from deploy.ts to avoid duplication.
 */

import fs from "fs";
import path from "path";

import {
  type Address,
  address,
  createSolanaRpc,
  createSolanaRpcSubscriptions,
  createKeyPairSignerFromBytes,
  pipe,
  createTransactionMessage,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  appendTransactionMessageInstructions,
  signTransactionMessageWithSigners,
  sendAndConfirmTransactionFactory,
  getSignatureFromTransaction,
  getAddressEncoder,
  AccountRole,
  type KeyPairSigner,
  type Rpc,
  type SolanaRpcApi,
  type RpcSubscriptions,
  type SolanaRpcSubscriptionsApi,
  type ReadonlyUint8Array,
} from "@solana/kit";

import {
  TOKEN_2022_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  SYSTEM_PROGRAM_ID,
} from "./constants.js";

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

export function rpcUrlToWsUrl(rpcUrl: string): string {
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
// Setup helper — loads config, creates RPC, loads admin keypair
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

const addressEncoder = getAddressEncoder();

/**
 * Derive the Associated Token Account address for Token-2022.
 * ATA PDA seeds: [wallet, token_program, mint] → ATA program
 */
export async function getAssociatedTokenAddress(
  wallet: Address,
  mint: Address
): Promise<Address> {
  const { getProgramDerivedAddress } = await import("@solana/kit");
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
 *
 * ATA program instruction index 1 = CreateIdempotent
 * Accounts:
 *   0. [signer, writable] Payer
 *   1. [writable]          Associated token account
 *   2. []                  Wallet (owner)
 *   3. []                  Token mint
 *   4. []                  System program
 *   5. []                  Token program (Token-2022)
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
    data: new Uint8Array([1]) as ReadonlyUint8Array, // instruction index 1 = CreateIdempotent
  };
}

// =================================================================
// Mint address helpers
// =================================================================

const __dirname = path.dirname(new URL(import.meta.url).pathname);
const KEYS_DIR = path.join(__dirname, "..", "keys");

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
