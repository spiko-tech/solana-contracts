import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

import {
  type Address,
  createSolanaRpc,
  createSolanaRpcSubscriptions,
  createKeyPairSignerFromBytes,
  createKeyPairSignerFromPrivateKeyBytes,
  pipe,
  createTransactionMessage,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  appendTransactionMessageInstructions,
  signTransactionMessageWithSigners,
  sendAndConfirmTransactionFactory,
  getSignatureFromTransaction,
  type KeyPairSigner,
  type Rpc,
  type SolanaRpcApi,
  type RpcSubscriptions,
  type SolanaRpcSubscriptionsApi,
} from "@solana/kit";

import { getCreateAccountInstruction } from "@solana-program/system";

import {
  PERMISSION_MANAGER_PROGRAM_ID,
  SPIKO_TOKEN_PROGRAM_ID,
  MINTER_PROGRAM_ID,
  REDEMPTION_PROGRAM_ID,
  TRANSFER_HOOK_PROGRAM_ID,
  TOKEN_2022_PROGRAM_ID,
  mintAccountSize,
  MINT_FIXED_EXTENSIONS_SIZE,
  MAX_DELAY,
  DAILY_LIMIT,
  REDEMPTION_MINIMUM,
  ALL_ROLES,
  ROLE_NAMES,
  ROLE_MINTER,
  ROLE_WHITELISTED,
  ROLE_BURNER,
} from "./lib/constants.js";

import {
  permissionConfigPda,
  userPermissionsPda,
  tokenConfigPda,
  mintAuthorityPda,
  extraAccountMetaListPda,
  minterConfigPda,
  dailyLimitPda,
  redemptionConfigPda,
  tokenMinimumPda,
  vaultAuthorityPda,
} from "./lib/pda.js";

import {
  initializePermissionManager,
  grantRole,
  initializeToken,
  initExtraAccountMetas,
  setRedemptionContract,
  initializeMinter,
  setDailyLimit,
  initializeRedemption,
  setMinimum,
} from "./lib/instructions.js";

// =================================================================
// Config
// =================================================================

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const KEYS_DIR = path.join(__dirname, "keys");

// Read Solana CLI config to get RPC URL and keypair path
function loadSolanaConfig(): { rpcUrl: string; keypairPath: string } {
  const configPath = path.join(
    process.env.HOME || "~",
    ".config/solana/cli/config.yml"
  );
  const content = fs.readFileSync(configPath, "utf-8");

  // Simple YAML parsing (no external dep)
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
// Helpers
// =================================================================

async function loadKeypair(filePath: string): Promise<KeyPairSigner> {
  const bytes = new Uint8Array(
    JSON.parse(fs.readFileSync(filePath, "utf-8"))
  );
  return createKeyPairSignerFromBytes(bytes);
}

/** Load or generate a mint keypair, saving to keys/ dir for idempotency. */
async function loadOrCreateMintKeypair(
  name: string
): Promise<KeyPairSigner> {
  const keyPath = path.join(KEYS_DIR, `${name}-mint.json`);

  if (fs.existsSync(keyPath)) {
    console.log(`  Loading existing ${name} mint keypair from ${keyPath}`);
    return loadKeypair(keyPath);
  }

  console.log(`  Generating new ${name} mint keypair...`);

  // Generate 32 random bytes for the private key
  const privateKeyBytes = new Uint8Array(32);
  crypto.getRandomValues(privateKeyBytes);

  // Create signer with extractable=true so we can export the public key
  const signer = await createKeyPairSignerFromPrivateKeyBytes(
    privateKeyBytes,
    /* extractable */ true
  );

  // Export public key to build 64-byte Solana CLI keypair format
  const publicKeyBytes = new Uint8Array(
    await crypto.subtle.exportKey("raw", signer.keyPair.publicKey)
  );

  // Solana CLI keypair format: 64 bytes = [privateKey(32), publicKey(32)]
  const keypairBytes = new Uint8Array(64);
  keypairBytes.set(privateKeyBytes, 0);
  keypairBytes.set(publicKeyBytes, 32);

  fs.mkdirSync(KEYS_DIR, { recursive: true });
  fs.writeFileSync(keyPath, JSON.stringify(Array.from(keypairBytes)));
  console.log(`  Saved ${name} mint keypair to ${keyPath}`);

  return signer;
}

async function accountExists(
  rpc: Rpc<SolanaRpcApi>,
  addr: Address
): Promise<boolean> {
  const { value } = await rpc
    .getAccountInfo(addr, { encoding: "base64" })
    .send();
  return value !== null;
}

async function sendTx(
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
// Main Deploy Script
// =================================================================

async function main() {
  console.log("=== Spiko Token System — Full Deployment ===\n");

  // --- Load config ---
  const config = loadSolanaConfig();
  console.log(`RPC URL:  ${config.rpcUrl}`);
  console.log(`Keypair:  ${config.keypairPath}\n`);

  const rpc = createSolanaRpc(config.rpcUrl);
  const rpcSub = createSolanaRpcSubscriptions(rpcUrlToWsUrl(config.rpcUrl));
  const admin = await loadKeypair(config.keypairPath);

  console.log(`Deployer: ${admin.address}`);
  const { value: balance } = await rpc.getBalance(admin.address).send();
  console.log(`Balance:  ${Number(balance) / 1e9} SOL\n`);

  if (balance === 0n) {
    console.error("ERROR: Deployer has 0 SOL. Fund the wallet first.");
    process.exit(1);
  }

  // --- Derive all PDAs ---
  console.log("Deriving PDAs...");
  const [permConfigAddr] = await permissionConfigPda();
  const [adminPermsAddr] = await userPermissionsPda(admin.address);
  const [minterConfigAddr] = await minterConfigPda();
  const [redemptionConfigAddr] = await redemptionConfigPda();

  console.log(`  PermissionConfig PDA:  ${permConfigAddr}`);
  console.log(`  Admin UserPerms PDA:   ${adminPermsAddr}`);
  console.log(`  MinterConfig PDA:      ${minterConfigAddr}`);
  console.log(`  RedemptionConfig PDA:  ${redemptionConfigAddr}`);
  console.log();

  // ===============================================================
  // Phase 1: Initialize Permission Manager
  // ===============================================================
  console.log("--- Phase 1: Initialize Permission Manager ---");

  if (await accountExists(rpc, permConfigAddr)) {
    console.log("  PermissionConfig already exists. Skipping.\n");
  } else {
    const ix = initializePermissionManager(
      admin,
      permConfigAddr,
      adminPermsAddr
    );
    await sendTx(rpc, rpcSub, admin, [ix], "InitializePermissionManager");
    console.log("  Done.\n");
  }

  // ===============================================================
  // Phase 2: Initialize Spiko Token (EUTBL + USTBL)
  // ===============================================================
  console.log("--- Phase 2: Initialize Spiko Token (EUTBL + USTBL) ---");

  // Per-fund token metadata
  const fundMetadata: Record<string, { name: string; symbol: string; uri: string; decimals: number }> = {
    eutbl: {
      name: "Spiko EU T-Bill",
      symbol: "EUTBL",
      uri: "https://spiko.finance/metadata/eutbl.json",
      decimals: 5,
    },
    ustbl: {
      name: "Spiko US T-Bill",
      symbol: "USTBL",
      uri: "https://spiko.finance/metadata/ustbl.json",
      decimals: 5,
    },
  };

  const fundNames = ["eutbl", "ustbl"] as const;
  const mintSigners: KeyPairSigner[] = [];
  const mintAddresses: Address[] = [];

  for (const name of fundNames) {
    console.log(`\n  [${name.toUpperCase()}]`);

    const meta = fundMetadata[name];
    const mintSigner = await loadOrCreateMintKeypair(name);
    mintSigners.push(mintSigner);
    mintAddresses.push(mintSigner.address);

    const [tokenConfigAddr] = await tokenConfigPda(mintSigner.address);
    const [mintAuthAddr] = await mintAuthorityPda(mintSigner.address);
    const finalAccountSize = mintAccountSize(meta.name, meta.symbol, meta.uri);

    console.log(`  Mint address:       ${mintSigner.address}`);
    console.log(`  TokenConfig PDA:    ${tokenConfigAddr}`);
    console.log(`  MintAuthority PDA:  ${mintAuthAddr}`);
    console.log(`  Mint account size:  ${MINT_FIXED_EXTENSIONS_SIZE} bytes (fixed) → ${finalAccountSize} bytes (after metadata realloc)`);
    console.log(`  Metadata:           ${meta.name} (${meta.symbol}), decimals=${meta.decimals}`);

    if (await accountExists(rpc, tokenConfigAddr)) {
      console.log("  TokenConfig already exists. Skipping.");
      continue;
    }

    // Create the mint account (owned by Token-2022) + Initialize token in one tx.
    // Space = fixed extensions only (338); InitializeMint2 validates exact size.
    // Lamports = rent-exempt for the final size (after TokenMetadataInitialize reallocs).
    const rentLamports = await rpc
      .getMinimumBalanceForRentExemption(finalAccountSize)
      .send();

    const createMintIx = getCreateAccountInstruction({
      payer: admin,
      newAccount: mintSigner,
      lamports: rentLamports,
      space: MINT_FIXED_EXTENSIONS_SIZE,
      programAddress: TOKEN_2022_PROGRAM_ID,
    });

    const initTokenIx = initializeToken(
      admin,
      tokenConfigAddr,
      mintSigner.address,
      mintAuthAddr,
      meta.decimals,
      meta.name,
      meta.symbol,
      meta.uri
    );

    await sendTx(
      rpc,
      rpcSub,
      admin,
      [createMintIx, initTokenIx],
      `InitializeToken(${name.toUpperCase()})`
    );
  }
  console.log("  Done.\n");

  // ===============================================================
  // Phase 2.5: Initialize ExtraAccountMetaList for Transfer Hook
  // ===============================================================
  console.log("--- Phase 2.5: Initialize ExtraAccountMetaList (Transfer Hook) ---");

  for (let i = 0; i < fundNames.length; i++) {
    const name = fundNames[i];
    const mintAddr = mintAddresses[i];
    console.log(`\n  [${name.toUpperCase()}]`);

    const [extraMetaListAddr] = await extraAccountMetaListPda(mintAddr);
    const [tokenConfigAddr] = await tokenConfigPda(mintAddr);

    console.log(`  ExtraAccountMetaList PDA: ${extraMetaListAddr}`);

    if (await accountExists(rpc, extraMetaListAddr)) {
      console.log("  ExtraAccountMetaList already exists. Skipping.");
      continue;
    }

    const ix = initExtraAccountMetas(
      admin,
      extraMetaListAddr,
      mintAddr,
      tokenConfigAddr
    );
    await sendTx(
      rpc,
      rpcSub,
      admin,
      [ix],
      `InitExtraAccountMetas(${name.toUpperCase()})`
    );
  }
  console.log("  Done.\n");

  // ===============================================================
  // Phase 3: Initialize Minter
  // ===============================================================
  console.log("--- Phase 3: Initialize Minter ---");

  if (await accountExists(rpc, minterConfigAddr)) {
    console.log("  MinterConfig already exists. Skipping.\n");
  } else {
    const ix = initializeMinter(
      admin,
      minterConfigAddr,
      MAX_DELAY,
      PERMISSION_MANAGER_PROGRAM_ID
    );
    await sendTx(rpc, rpcSub, admin, [ix], "InitializeMinter");
    console.log("  Done.\n");
  }

  // ===============================================================
  // Phase 4: Initialize Redemption
  // ===============================================================
  console.log("--- Phase 4: Initialize Redemption ---");

  if (await accountExists(rpc, redemptionConfigAddr)) {
    console.log("  RedemptionConfig already exists. Skipping.\n");
  } else {
    const ix = initializeRedemption(
      admin,
      redemptionConfigAddr,
      PERMISSION_MANAGER_PROGRAM_ID
    );
    await sendTx(rpc, rpcSub, admin, [ix], "InitializeRedemption");
    console.log("  Done.\n");
  }

  // ===============================================================
  // Phase 5: Link Programs
  // ===============================================================
  console.log("--- Phase 5: Link Programs ---");

  for (let i = 0; i < fundNames.length; i++) {
    const name = fundNames[i];
    const mintAddr = mintAddresses[i];
    console.log(`\n  [${name.toUpperCase()}]`);

    const [tokenConfigAddr] = await tokenConfigPda(mintAddr);
    const [dailyLimitAddr] = await dailyLimitPda(mintAddr);
    const [tokenMinAddr] = await tokenMinimumPda(mintAddr);

    // 5a: SetRedemptionContract on spiko-token
    console.log("  Setting redemption contract...");
    const setRedemptionIx = setRedemptionContract(
      admin,
      tokenConfigAddr,
      permConfigAddr,
      REDEMPTION_PROGRAM_ID
    );
    await sendTx(
      rpc,
      rpcSub,
      admin,
      [setRedemptionIx],
      `SetRedemptionContract(${name.toUpperCase()})`
    );

    // 5b: SetDailyLimit on minter
    console.log("  Setting daily limit...");
    const setLimitIx = setDailyLimit(
      admin,
      minterConfigAddr,
      permConfigAddr,
      dailyLimitAddr,
      mintAddr,
      DAILY_LIMIT
    );
    await sendTx(
      rpc,
      rpcSub,
      admin,
      [setLimitIx],
      `SetDailyLimit(${name.toUpperCase()})`
    );

    // 5c: SetMinimum on redemption
    console.log("  Setting redemption minimum...");
    const setMinIx = setMinimum(
      admin,
      redemptionConfigAddr,
      permConfigAddr,
      tokenMinAddr,
      mintAddr,
      REDEMPTION_MINIMUM
    );
    await sendTx(
      rpc,
      rpcSub,
      admin,
      [setMinIx],
      `SetMinimum(${name.toUpperCase()})`
    );
  }
  console.log("  Done.\n");

  // ===============================================================
  // Phase 6: Grant All Roles to Deployer
  // ===============================================================
  console.log("--- Phase 6: Grant All Roles to Deployer ---");

  for (const roleId of ALL_ROLES) {
    const roleName = ROLE_NAMES[roleId];
    console.log(`  Granting ${roleName} (bit ${roleId})...`);

    const ix = grantRole(
      admin,
      permConfigAddr,
      adminPermsAddr,
      admin.address,
      adminPermsAddr, // caller's perms = admin's perms (admin is calling)
      roleId
    );

    await sendTx(
      rpc,
      rpcSub,
      admin,
      [ix],
      `GrantRole(${roleName})`
    );
  }
  console.log("  Done.\n");

  // ===============================================================
  // Phase 7: Grant ROLE_MINTER to MinterConfig PDA
  // ===============================================================
  // The Minter program CPIs into SpikoToken.mint() signing as MinterConfig PDA.
  // SpikoToken checks that the CPI caller has ROLE_MINTER. Without this, no
  // minting through the Minter program can succeed.
  console.log("--- Phase 7: Grant ROLE_MINTER to MinterConfig PDA ---");

  const [minterConfigPermsAddr] = await userPermissionsPda(minterConfigAddr);
  console.log(`  MinterConfig UserPerms PDA: ${minterConfigPermsAddr}`);

  // Check if MinterConfig already has a UserPermissions PDA with data
  if (await accountExists(rpc, minterConfigPermsAddr)) {
    console.log("  MinterConfig UserPerms already exists. Skipping.\n");
  } else {
    const ix = grantRole(
      admin,
      permConfigAddr,
      minterConfigPermsAddr,
      minterConfigAddr,
      adminPermsAddr,
      ROLE_MINTER
    );
    await sendTx(rpc, rpcSub, admin, [ix], "GrantRole(MINTER → MinterConfig)");
    console.log("  Done.\n");
  }

  // ===============================================================
  // Phase 8: Grant Vault Authority PDA ROLE_WHITELISTED + ROLE_BURNER
  // ===============================================================
  // The vault authority PDA (seeds ["vault"] → REDEMPTION_PROGRAM_ID) needs:
  //   - ROLE_WHITELISTED (bit 4): so it can receive tokens via TransferChecked during redeem
  //   - ROLE_BURNER (bit 2): so spiko_token::burn accepts it as the caller during execute_redemption
  console.log("--- Phase 8: Grant Vault Authority PDA Roles ---");

  const [vaultAuthAddr] = await vaultAuthorityPda();
  const [vaultAuthPermsAddr] = await userPermissionsPda(vaultAuthAddr);
  console.log(`  Vault Authority PDA:       ${vaultAuthAddr}`);
  console.log(`  Vault Authority Perms PDA: ${vaultAuthPermsAddr}`);

  const vaultRoles = [
    { id: ROLE_WHITELISTED, name: "WHITELISTED" },
    { id: ROLE_BURNER, name: "BURNER" },
  ];

  for (const role of vaultRoles) {
    console.log(`  Granting ${role.name} (bit ${role.id}) to vault authority...`);
    const ix = grantRole(
      admin,
      permConfigAddr,
      vaultAuthPermsAddr,
      vaultAuthAddr,
      adminPermsAddr,
      role.id
    );
    await sendTx(
      rpc,
      rpcSub,
      admin,
      [ix],
      `GrantRole(${role.name} → VaultAuthority)`
    );
  }
  console.log("  Done.\n");

  // ===============================================================
  // Summary
  // ===============================================================
  console.log("=== Deployment Summary ===\n");
  console.log("Programs:");
  console.log(
    `  PermissionManager: ${PERMISSION_MANAGER_PROGRAM_ID}`
  );
  console.log(`  SpikoToken:        ${SPIKO_TOKEN_PROGRAM_ID}`);
  console.log(`  Minter:            ${MINTER_PROGRAM_ID}`);
  console.log(`  Redemption:        ${REDEMPTION_PROGRAM_ID}`);
  console.log(`  TransferHook:      ${TRANSFER_HOOK_PROGRAM_ID}`);

  console.log("\nPDAs:");
  console.log(`  PermissionConfig:  ${permConfigAddr}`);
  console.log(`  MinterConfig:      ${minterConfigAddr}`);
  console.log(`  MinterConfig Perms:${minterConfigPermsAddr}`);
  console.log(`  RedemptionConfig:  ${redemptionConfigAddr}`);
  console.log(`  VaultAuthority:    ${vaultAuthAddr}`);
  console.log(`  VaultAuth Perms:   ${vaultAuthPermsAddr}`);

  console.log("\nFund Tokens:");
  for (let i = 0; i < fundNames.length; i++) {
    const name = fundNames[i];
    const mintAddr = mintAddresses[i];
    const [tokenConfigAddr] = await tokenConfigPda(mintAddr);
    const [dailyLimitAddr] = await dailyLimitPda(mintAddr);
    const [tokenMinAddr] = await tokenMinimumPda(mintAddr);
    const [extraMetaListAddr] = await extraAccountMetaListPda(mintAddr);

    console.log(`\n  ${name.toUpperCase()}:`);
    console.log(`    Mint:             ${mintAddr}`);
    console.log(`    TokenConfig:      ${tokenConfigAddr}`);
    console.log(`    DailyLimit:       ${dailyLimitAddr}`);
    console.log(`    TokenMinimum:     ${tokenMinAddr}`);
    console.log(`    ExtraMetaList:    ${extraMetaListAddr}`);
  }

  console.log("\nAdmin:");
  console.log(`  Address:           ${admin.address}`);
  console.log(`  UserPerms PDA:     ${adminPermsAddr}`);
  console.log(`  Roles:             ALL (bits 0-7)`);

  console.log("\nParameters:");
  console.log(`  Max Delay:         ${MAX_DELAY}s (${Number(MAX_DELAY) / 3600}h)`);
  console.log(`  Daily Limit:       ${DAILY_LIMIT} (${Number(DAILY_LIMIT) / 1e5} shares)`);
  console.log(`  Redemption Min:    ${REDEMPTION_MINIMUM} (${Number(REDEMPTION_MINIMUM) / 1e5} shares)`);

  console.log("\n=== Deployment Complete ===");
}

main().catch((err) => {
  console.error("\nDeployment failed:", err);
  process.exit(1);
});
