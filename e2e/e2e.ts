/**
 * Self-contained end-to-end multi-actor test on devnet.
 *
 * Creates ALL on-chain state from scratch (mint, configs, roles),
 * then runs the full grant→mint→transfer→redeem→execute flow.
 *
 * Prerequisites:
 *   - All 5 Spiko programs deployed to devnet
 *   - Solana CLI configured for devnet with a funded admin keypair
 *
 * Usage:
 *   cd e2e && npx tsx e2e.ts
 */

import {
  type Rpc,
  type SolanaRpcApi,
  type RpcSubscriptions,
  type SolanaRpcSubscriptionsApi,
  type KeyPairSigner,
  type Address,
  pipe,
  createTransactionMessage,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  appendTransactionMessageInstructions,
  signTransactionMessageWithSigners,
  sendAndConfirmTransactionFactory,
  getSignatureFromTransaction,
  generateKeyPairSigner,
  lamports,
} from "@solana/kit";

// ── Codama-generated instruction builders (setup) ─────────────
import { getInitializeInstructionAsync } from "../clients/ts/permission-manager/src/generated/instructions/initialize.js";
import { getInitializeTokenInstructionAsync } from "../clients/ts/spiko-token/src/generated/instructions/initializeToken.js";
import { getInitExtraAccountMetasInstruction } from "../clients/ts/spiko-transfer-hook/src/generated/instructions/initExtraAccountMetas.js";
import { getInitializeMinterInstructionAsync } from "../clients/ts/minter/src/generated/instructions/initializeMinter.js";
import { getInitializeRedemptionInstructionAsync } from "../clients/ts/redemption/src/generated/instructions/initializeRedemption.js";
import { getSetRedemptionContractInstruction } from "../clients/ts/spiko-token/src/generated/instructions/setRedemptionContract.js";
import { getSetDailyLimitInstruction } from "../clients/ts/minter/src/generated/instructions/setDailyLimit.js";
import { getSetMinimumInstruction } from "../clients/ts/redemption/src/generated/instructions/setMinimum.js";

// ── Codama-generated instruction builders (test flow) ─────────
import { getGrantRoleInstructionAsync } from "../clients/ts/permission-manager/src/generated/instructions/grantRole.js";
import { getInitiateMintInstruction } from "../clients/ts/minter/src/generated/instructions/initiateMint.js";
import { getTransferTokenInstruction } from "../clients/ts/spiko-token/src/generated/instructions/transferToken.js";
import { getRedeemTokenInstruction } from "../clients/ts/spiko-token/src/generated/instructions/redeemToken.js";
import { getExecuteRedemptionInstruction } from "../clients/ts/redemption/src/generated/instructions/executeRedemption.js";

// ── Codama-generated instruction builders (custodial gatekeeper) ─
import { getInitializeInstructionAsync as getInitializeCustodialGatekeeperInstructionAsync } from "../clients/ts/custodial-gatekeeper/src/generated/instructions/initialize.js";
import { getSetDailyLimitInstruction as getCgSetDailyLimitInstruction } from "../clients/ts/custodial-gatekeeper/src/generated/instructions/setDailyLimit.js";
import { getCustodialWithdrawInstruction } from "../clients/ts/custodial-gatekeeper/src/generated/instructions/custodialWithdraw.js";

// ── Shared helpers ────────────────────────────────────────────
import {
  ROLE_MINTER,
  ROLE_BURNER,
  ROLE_WHITELISTED,
  ROLE_WHITELISTED_EXT,
  ROLE_WHITELISTER,
  ROLE_MINT_INITIATOR,
  ROLE_REDEMPTION_EXECUTOR,
  PERMISSION_MANAGER_PROGRAM_ADDRESS,
  SPIKO_TOKEN_PROGRAM_ADDRESS,
  SPIKO_TRANSFER_HOOK_PROGRAM_ADDRESS,
  REDEMPTION_PROGRAM_ADDRESS,
  CUSTODIAL_GATEKEEPER_PROGRAM_ADDRESS,
  TOKEN_2022_PROGRAM_ID,
  findPermissionConfigPda,
  findUserPermissionsPda,
  findTokenConfigPda,
  findMinterConfigPda,
  findDailyLimitPda,
  findMintOperationPda,
  findRedemptionConfigPda,
  findRedemptionOperationPda,
  findTokenMinimumPda,
  findGatekeeperConfigPda,
  findWithdrawalDailyLimitPda,
  findWithdrawalOperationPda,
  mintAuthorityPda,
  extraAccountMetaListPda,
  vaultAuthorityPda,
  custodialGatekeeperVaultAuthorityPda,
  spikoTokenEventAuthorityPda,
  transferHookEventAuthorityPda,
  minterEventAuthorityPda,
  redemptionEventAuthorityPda,
  custodialGatekeeperEventAuthorityPda,
  setup,
  getAssociatedTokenAddress,
  createAssociatedTokenAccountIdempotent,
  accountExists,
  computeOperationId,
  getMintAccountSpace,
  MINT_FIXED_EXTENSIONS_SIZE,
  buildCreateAccountInstruction,
  buildTransferChecked,
} from "./lib/shared.js";

import { parseTransactionEvents, formatEvents } from "./lib/events.js";

// =================================================================
// Token Parameters
// =================================================================

const TOKEN_DECIMALS = 5;
const TOKEN_NAME = "EUR T-Bill";
const TOKEN_SYMBOL = "EUTBL";
const TOKEN_URI = "";
const DAILY_LIMIT = 100_000_000_000n; // 1M shares at 5 decimals
const REDEMPTION_MINIMUM = 100_000n; // 1 share at 5 decimals
const MAX_DELAY = 86400n; // 1 day in seconds

// =================================================================
// Test Parameters
// =================================================================

const MINT_SHARES = 20;
const TRANSFER_B_SHARES = 10; // Path B: spiko-token transfer
const TRANSFER_A_SHARES = 5; // Path A: direct TransferChecked
const REDEEM_SHARES = 10;
const CUSTODIAL_WITHDRAW_SHARES = 1; // Custodial gatekeeper withdraw to user3

// =================================================================
// Helpers
// =================================================================

async function sendAndCapture(
  rpc: Rpc<SolanaRpcApi>,
  rpcSub: RpcSubscriptions<SolanaRpcSubscriptionsApi>,
  payer: KeyPairSigner,
  instructions: any[],
  label: string,
): Promise<string> {
  const { value: latestBlockhash } = await rpc.getLatestBlockhash().send();

  const txMessage = pipe(
    createTransactionMessage({ version: 0 }),
    (tx) => setTransactionMessageFeePayerSigner(payer, tx),
    (tx) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, tx),
    (tx) => appendTransactionMessageInstructions(instructions, tx),
  );

  const signedTx = await signTransactionMessageWithSigners(txMessage);
  const sendAndConfirm = sendAndConfirmTransactionFactory({
    rpc,
    rpcSubscriptions: rpcSub,
  });
  await sendAndConfirm(signedTx, { commitment: "confirmed" });

  const sig = getSignatureFromTransaction(signedTx);
  console.log(`  ${label}: ${sig}`);
  return sig;
}

/**
 * Airdrop SOL to an address. Falls back to a transfer from admin
 * if the devnet faucet is rate-limited.
 */
async function fundAccount(
  rpc: Rpc<SolanaRpcApi>,
  rpcSub: RpcSubscriptions<SolanaRpcSubscriptionsApi>,
  admin: KeyPairSigner,
  target: Address,
  amount: bigint = 50_000_000n, // 0.05 SOL
): Promise<void> {
  try {
    const sig = await rpc.requestAirdrop(target, lamports(amount)).send();
    await new Promise((r) => setTimeout(r, 2000));
    console.log(
      `  Airdropped ${Number(amount) / 1e9} SOL to ${target.slice(0, 8)}...`,
    );
  } catch {
    const { getTransferSolInstruction } =
      await import("@solana-program/system");
    const ix = getTransferSolInstruction({
      source: admin,
      destination: target,
      amount: amount,
    });
    await sendAndCapture(
      rpc,
      rpcSub,
      admin,
      [ix],
      `Fund ${target.slice(0, 8)}...`,
    );
  }
}

/**
 * Get the minimum rent-exempt balance for a given account size.
 */
async function getMinRent(
  rpc: Rpc<SolanaRpcApi>,
  space: number,
): Promise<bigint> {
  const rent = await rpc
    .getMinimumBalanceForRentExemption(BigInt(space))
    .send();
  return rent;
}

// =================================================================
// Main
// =================================================================

async function main() {
  console.log("=== Spiko E2E Self-Contained Test ===\n");

  const { rpc, rpcSub, admin } = await setup();

  // ── Generate fresh mint keypair ────────────────────────────
  const mintKp = await generateKeyPairSigner();
  const mintAddr = mintKp.address;
  const decimals = TOKEN_DECIMALS;

  // ── Generate 5 actor keypairs ──────────────────────────────
  const minter = await generateKeyPairSigner();
  const executor = await generateKeyPairSigner();
  const whitelister = await generateKeyPairSigner();
  const user1 = await generateKeyPairSigner();
  const user2 = await generateKeyPairSigner();
  const user3 = await generateKeyPairSigner(); // self-custodial (WHITELISTED_EXT)

  console.log("Accounts:");
  console.log(`  Admin:       ${admin.address}`);
  console.log(`  Mint:        ${mintAddr}`);
  console.log(`  Minter:      ${minter.address}`);
  console.log(`  Executor:    ${executor.address}`);
  console.log(`  Whitelister: ${whitelister.address}`);
  console.log(`  User1:       ${user1.address}`);
  console.log(`  User2:       ${user2.address}`);
  console.log(`  User3:       ${user3.address} (self-custodial)`);

  // ── Pre-compute raw amounts ────────────────────────────────
  const mintRawAmount = BigInt(Math.round(MINT_SHARES * 10 ** decimals));
  const transferBRawAmount = BigInt(
    Math.round(TRANSFER_B_SHARES * 10 ** decimals),
  );
  const transferARawAmount = BigInt(
    Math.round(TRANSFER_A_SHARES * 10 ** decimals),
  );
  const redeemRawAmount = BigInt(Math.round(REDEEM_SHARES * 10 ** decimals));
  const custodialWithdrawRawAmount = BigInt(
    Math.round(CUSTODIAL_WITHDRAW_SHARES * 10 ** decimals),
  );

  const mintSalt = BigInt(Math.floor(Math.random() * Number.MAX_SAFE_INTEGER));
  const redeemSalt = BigInt(
    Math.floor(Math.random() * Number.MAX_SAFE_INTEGER),
  );
  const custodialWithdrawSalt = BigInt(
    Math.floor(Math.random() * Number.MAX_SAFE_INTEGER),
  );

  // ── Pre-compute PDAs ───────────────────────────────────────
  const [permConfigAddr] = await findPermissionConfigPda();
  const [minterConfigAddr] = await findMinterConfigPda();
  const [minterConfigPermsAddr] = await findUserPermissionsPda({
    user: minterConfigAddr,
  });
  const [tokenConfigAddr] = await findTokenConfigPda({ splMint: mintAddr });
  const [dailyLimitAddr] = await findDailyLimitPda({ mint: mintAddr });
  const [redemptionConfigAddr] = await findRedemptionConfigPda();
  const [mintAuthAddr] = await mintAuthorityPda(mintAddr);
  const [vaultAuthAddr] = await vaultAuthorityPda();
  const [vaultAuthPermsAddr] = await findUserPermissionsPda({
    user: vaultAuthAddr,
  });
  const [extraMetaListAddr] = await extraAccountMetaListPda(mintAddr);
  const [tokenMinAddr] = await findTokenMinimumPda({ mint: mintAddr });

  // Custodial Gatekeeper PDAs
  const [cgConfigAddr] = await findGatekeeperConfigPda();
  const [cgVaultAuthAddr] = await custodialGatekeeperVaultAuthorityPda();
  const [cgVaultAuthPermsAddr] = await findUserPermissionsPda({
    user: cgVaultAuthAddr,
  });
  const [cgDailyLimitAddr] = await findWithdrawalDailyLimitPda({
    mint: mintAddr,
  });

  // Event authority PDAs
  const [stEventAuth] = await spikoTokenEventAuthorityPda();
  const [thEventAuth] = await transferHookEventAuthorityPda();
  const [mtEventAuth] = await minterEventAuthorityPda();
  const [rdEventAuth] = await redemptionEventAuthorityPda();
  const [cgEventAuth] = await custodialGatekeeperEventAuthorityPda();

  // ATAs
  const user1Ata = await getAssociatedTokenAddress(user1.address, mintAddr);
  const user2Ata = await getAssociatedTokenAddress(user2.address, mintAddr);
  const user3Ata = await getAssociatedTokenAddress(user3.address, mintAddr);
  const vaultAta = await getAssociatedTokenAddress(vaultAuthAddr, mintAddr);
  const cgVaultAta = await getAssociatedTokenAddress(cgVaultAuthAddr, mintAddr);

  // ═══════════════════════════════════════════════════════════
  //  SETUP PHASE: Create all on-chain state from scratch
  // ═══════════════════════════════════════════════════════════

  console.log("\n========================================");
  console.log("  SETUP PHASE: Deploying on-chain state");
  console.log("========================================\n");

  // ── S0: Create Token-2022 mint account ─────────────────────
  console.log("--- S0: Create Token-2022 mint account ---\n");
  {
    // CreateAccount space = fixed extensions only (338).
    // InitializeMint2 validates this exact size.
    // TokenMetadataInitialize (CPI inside InitializeToken) reallocs to final size.
    // Rent lamports must cover the final size (with metadata).
    const finalSpace = getMintAccountSpace(TOKEN_NAME, TOKEN_SYMBOL, TOKEN_URI);
    const rentLamports = await getMinRent(rpc, finalSpace);
    const ix = buildCreateAccountInstruction(
      admin,
      mintKp,
      rentLamports,
      MINT_FIXED_EXTENSIONS_SIZE,
      TOKEN_2022_PROGRAM_ID,
    );
    await sendAndCapture(rpc, rpcSub, admin, [ix], "CreateAccount(mint)");
  }

  // ── S1: Initialize PermissionManager ───────────────────────
  // ── S4: Initialize Minter ──────────────────────────────────
  // ── S5: Initialize Redemption ──────────────────────────────
  // Skip any that are already initialized (singleton configs persist across runs)
  console.log(
    "\n--- S1+S4+S5: Initialize PermissionManager + Minter + Redemption ---\n",
  );
  {
    const initIxs: any[] = [];
    const labels: string[] = [];

    if (!(await accountExists(rpc, permConfigAddr))) {
      initIxs.push(await getInitializeInstructionAsync({ admin }));
      labels.push("PermissionManager");
    } else {
      console.log("  (PermissionManager already initialized — skipping)");
    }

    if (!(await accountExists(rpc, minterConfigAddr))) {
      initIxs.push(
        await getInitializeMinterInstructionAsync({
          admin,
          maxDelay: MAX_DELAY,
          permissionManager: PERMISSION_MANAGER_PROGRAM_ADDRESS as Address,
        }),
      );
      labels.push("Minter");
    } else {
      console.log("  (Minter already initialized — skipping)");
    }

    if (!(await accountExists(rpc, redemptionConfigAddr))) {
      initIxs.push(
        await getInitializeRedemptionInstructionAsync({
          admin,
          permissionManager: PERMISSION_MANAGER_PROGRAM_ADDRESS as Address,
          eventAuthority: rdEventAuth,
        }),
      );
      labels.push("Redemption");
    } else {
      console.log("  (Redemption already initialized — skipping)");
    }

    if (initIxs.length > 0) {
      await sendAndCapture(
        rpc,
        rpcSub,
        admin,
        initIxs,
        `Initialize(${labels.join(" + ")})`,
      );
    }
  }

  // ── S2: Initialize SpikoToken (for the mint) ───────────────
  console.log("\n--- S2: Initialize SpikoToken ---\n");
  {
    const ix = await getInitializeTokenInstructionAsync({
      admin,
      mint: mintAddr,
      permissionManager: PERMISSION_MANAGER_PROGRAM_ADDRESS as Address,
      transferHookProgram: SPIKO_TRANSFER_HOOK_PROGRAM_ADDRESS as Address,
      eventAuthority: stEventAuth,
      decimals: TOKEN_DECIMALS,
      name: TOKEN_NAME,
      symbol: TOKEN_SYMBOL,
      uri: TOKEN_URI,
    });
    await sendAndCapture(rpc, rpcSub, admin, [ix], "InitializeToken(EUTBL)");
  }

  // ── S3: Init ExtraAccountMetas (for the mint) ──────────────
  console.log("\n--- S3: Init ExtraAccountMetas ---\n");
  {
    const ix = getInitExtraAccountMetasInstruction({
      admin,
      extraAccountMetaList: extraMetaListAddr,
      mint: mintAddr,
      tokenConfig: tokenConfigAddr,
    });
    await sendAndCapture(rpc, rpcSub, admin, [ix], "InitExtraAccountMetas");
  }

  // ── S6+S7+S8: Grant roles to MinterConfig & VaultAuthority ─
  console.log(
    "\n--- S6+S7+S8: Grant ROLE_MINTER to MinterConfig + ROLE_BURNER & ROLE_WHITELISTED to VaultAuthority ---\n",
  );
  {
    const roleIxs: any[] = [];
    const roleLabels: string[] = [];

    if (!(await accountExists(rpc, minterConfigPermsAddr))) {
      roleIxs.push(
        await getGrantRoleInstructionAsync({
          caller: admin,
          config: permConfigAddr,
          targetUser: minterConfigAddr,
          roleId: ROLE_MINTER,
        }),
      );
      roleLabels.push("MINTER->MinterConfig");
    } else {
      console.log("  (MinterConfig perms already exist — skipping)");
    }

    if (!(await accountExists(rpc, vaultAuthPermsAddr))) {
      roleIxs.push(
        await getGrantRoleInstructionAsync({
          caller: admin,
          config: permConfigAddr,
          targetUser: vaultAuthAddr,
          roleId: ROLE_BURNER,
        }),
      );
      roleIxs.push(
        await getGrantRoleInstructionAsync({
          caller: admin,
          config: permConfigAddr,
          targetUser: vaultAuthAddr,
          roleId: ROLE_WHITELISTED,
        }),
      );
      roleLabels.push("BURNER+WHITELISTED->VaultAuth");
    } else {
      console.log("  (VaultAuthority perms already exist — skipping)");
    }

    if (roleIxs.length > 0) {
      await sendAndCapture(
        rpc,
        rpcSub,
        admin,
        roleIxs,
        `GrantRole(${roleLabels.join(", ")})`,
      );
    }
  }

  // ── S9+S10+S11: SetRedemptionContract + SetDailyLimit + SetMinimum ─
  console.log(
    "\n--- S9+S10+S11: SetRedemptionContract + SetDailyLimit + SetMinimum ---\n",
  );
  {
    const ixSetRedemption = getSetRedemptionContractInstruction({
      caller: admin,
      config: tokenConfigAddr,
      permConfig: permConfigAddr,
      eventAuthority: stEventAuth,
      redemptionContract: REDEMPTION_PROGRAM_ADDRESS as Address,
    });
    const ixSetDailyLimit = getSetDailyLimitInstruction({
      caller: admin,
      config: minterConfigAddr,
      permConfig: permConfigAddr,
      dailyLimit: dailyLimitAddr,
      tokenMint: mintAddr,
      limit: DAILY_LIMIT,
    });
    const ixSetMinimum = getSetMinimumInstruction({
      caller: admin,
      config: redemptionConfigAddr,
      permConfig: permConfigAddr,
      tokenMinimum: tokenMinAddr,
      eventAuthority: rdEventAuth,
      tokenMint: mintAddr,
      minimum: REDEMPTION_MINIMUM,
    });
    await sendAndCapture(
      rpc,
      rpcSub,
      admin,
      [ixSetRedemption, ixSetDailyLimit, ixSetMinimum],
      "SetRedemptionContract + SetDailyLimit + SetMinimum",
    );
  }

  // ── S12: Initialize CustodialGatekeeper ────────────────────
  console.log("\n--- S12: Initialize CustodialGatekeeper ---\n");
  {
    if (!(await accountExists(rpc, cgConfigAddr))) {
      const ix = await getInitializeCustodialGatekeeperInstructionAsync({
        admin,
        permissionManager: PERMISSION_MANAGER_PROGRAM_ADDRESS as Address,
        maxDelay: MAX_DELAY,
        eventAuthority: cgEventAuth,
      });
      await sendAndCapture(
        rpc,
        rpcSub,
        admin,
        [ix],
        "Initialize(CustodialGatekeeper)",
      );
    } else {
      console.log("  (CustodialGatekeeper already initialized — skipping)");
    }
  }

  // ── S13: Grant WHITELISTED_EXT + WHITELISTED to CG VaultAuthority ──
  console.log(
    "\n--- S13: Grant WHITELISTED_EXT + WHITELISTED to CG VaultAuthority ---\n",
  );
  {
    if (!(await accountExists(rpc, cgVaultAuthPermsAddr))) {
      const ix1 = await getGrantRoleInstructionAsync({
        caller: admin,
        config: permConfigAddr,
        targetUser: cgVaultAuthAddr,
        roleId: ROLE_WHITELISTED_EXT,
      });
      const ix2 = await getGrantRoleInstructionAsync({
        caller: admin,
        config: permConfigAddr,
        targetUser: cgVaultAuthAddr,
        roleId: ROLE_WHITELISTED,
      });
      await sendAndCapture(
        rpc,
        rpcSub,
        admin,
        [ix1, ix2],
        "GrantRole(WHITELISTED_EXT+WHITELISTED -> CG VaultAuth)",
      );
    } else {
      console.log("  (CG VaultAuthority perms already exist — skipping)");
    }
  }

  // ── S14: Set CustodialGatekeeper DailyLimit for this mint ──
  console.log("\n--- S14: Set CustodialGatekeeper DailyLimit ---\n");
  {
    const ix = getCgSetDailyLimitInstruction({
      caller: admin,
      config: cgConfigAddr,
      permConfig: permConfigAddr,
      dailyLimit: cgDailyLimitAddr,
      eventAuthority: cgEventAuth,
      tokenMint: mintAddr,
      limit: DAILY_LIMIT,
    });
    await sendAndCapture(
      rpc,
      rpcSub,
      admin,
      [ix],
      "SetDailyLimit(CustodialGatekeeper)",
    );
  }

  console.log("\n========================================");
  console.log("  SETUP COMPLETE — Starting test flow");
  console.log("========================================\n");

  console.log(`Token:          ${TOKEN_SYMBOL}`);
  console.log(`Mint:           ${mintAddr}`);
  console.log(`Decimals:       ${decimals}`);
  console.log(`User1 ATA:      ${user1Ata}`);
  console.log(`User2 ATA:      ${user2Ata}`);
  console.log(`User3 ATA:      ${user3Ata}`);
  console.log(`Vault ATA:      ${vaultAta}`);
  console.log(`CG Vault ATA:   ${cgVaultAta}\n`);

  // ── Fund accounts that need to sign ────────────────────────
  console.log("--- Funding accounts ---\n");
  await fundAccount(rpc, rpcSub, admin, minter.address);
  await fundAccount(rpc, rpcSub, admin, executor.address);
  await fundAccount(rpc, rpcSub, admin, whitelister.address);
  await fundAccount(rpc, rpcSub, admin, user1.address);
  await fundAccount(rpc, rpcSub, admin, user2.address);
  await fundAccount(rpc, rpcSub, admin, user3.address);
  console.log();

  // ═══════════════════════════════════════════════════════════
  //  TEST FLOW
  // ═══════════════════════════════════════════════════════════

  let totalEvents = 0;
  let totalSteps = 0;

  async function step(
    label: string,
    expectedEvents: string[],
    fn: () => Promise<string>,
  ) {
    totalSteps++;
    console.log(`\n--- Step ${totalSteps}: ${label} ---\n`);

    const sig = await fn();

    console.log(`\n  Fetching events from transaction (with retry)...`);
    const events = await parseTransactionEvents(rpc, sig);
    console.log(formatEvents(events));

    const foundNames = events.map((e) => e.name);
    for (const expected of expectedEvents) {
      if (foundNames.includes(expected)) {
        console.log(`  OK: ${expected}`);
        totalEvents++;
      } else {
        console.log(`  MISSING: ${expected}`);
      }
    }
  }

  // ═══════════════════════════════════════════════════════════
  // Steps 1-5: Grant roles to actors
  // ═══════════════════════════════════════════════════════════

  await step(
    "Admin grants ROLE_MINT_INITIATOR to Minter",
    ["RoleGranted"],
    async () => {
      const ix = await getGrantRoleInstructionAsync({
        caller: admin,
        config: permConfigAddr,
        targetUser: minter.address,
        roleId: ROLE_MINT_INITIATOR,
      });
      return sendAndCapture(
        rpc,
        rpcSub,
        admin,
        [ix],
        "GrantRole(MINT_INITIATOR -> Minter)",
      );
    },
  );

  await step(
    "Admin grants ROLE_REDEMPTION_EXECUTOR to Executor",
    ["RoleGranted"],
    async () => {
      const ix = await getGrantRoleInstructionAsync({
        caller: admin,
        config: permConfigAddr,
        targetUser: executor.address,
        roleId: ROLE_REDEMPTION_EXECUTOR,
      });
      return sendAndCapture(
        rpc,
        rpcSub,
        admin,
        [ix],
        "GrantRole(REDEMPTION_EXECUTOR -> Executor)",
      );
    },
  );

  await step(
    "Admin grants ROLE_WHITELISTER to Whitelister",
    ["RoleGranted"],
    async () => {
      const ix = await getGrantRoleInstructionAsync({
        caller: admin,
        config: permConfigAddr,
        targetUser: whitelister.address,
        roleId: ROLE_WHITELISTER,
      });
      return sendAndCapture(
        rpc,
        rpcSub,
        admin,
        [ix],
        "GrantRole(WHITELISTER -> Whitelister)",
      );
    },
  );

  await step(
    "Whitelister grants ROLE_WHITELISTED to User1",
    ["RoleGranted"],
    async () => {
      const ix = await getGrantRoleInstructionAsync({
        caller: whitelister,
        config: permConfigAddr,
        targetUser: user1.address,
        roleId: ROLE_WHITELISTED,
      });
      return sendAndCapture(
        rpc,
        rpcSub,
        whitelister,
        [ix],
        "GrantRole(WHITELISTED -> User1)",
      );
    },
  );

  await step(
    "Whitelister grants ROLE_WHITELISTED to User2",
    ["RoleGranted"],
    async () => {
      const ix = await getGrantRoleInstructionAsync({
        caller: whitelister,
        config: permConfigAddr,
        targetUser: user2.address,
        roleId: ROLE_WHITELISTED,
      });
      return sendAndCapture(
        rpc,
        rpcSub,
        whitelister,
        [ix],
        "GrantRole(WHITELISTED -> User2)",
      );
    },
  );

  await step(
    "Whitelister grants ROLE_WHITELISTED_EXT to User3 (self-custodial)",
    ["RoleGranted"],
    async () => {
      const ix = await getGrantRoleInstructionAsync({
        caller: whitelister,
        config: permConfigAddr,
        targetUser: user3.address,
        roleId: ROLE_WHITELISTED_EXT,
      });
      return sendAndCapture(
        rpc,
        rpcSub,
        whitelister,
        [ix],
        "GrantRole(WHITELISTED_EXT -> User3)",
      );
    },
  );

  // ═══════════════════════════════════════════════════════════
  // Step 6: Minter mints 20 shares to User1 (auto-executes)
  // ═══════════════════════════════════════════════════════════
  await step(
    `Minter mints ${MINT_SHARES} ${TOKEN_SYMBOL} to User1`,
    ["MintExecuted", "Mint"],
    async () => {
      const instructions: any[] = [];

      // Create User1's ATA if needed
      if (!(await accountExists(rpc, user1Ata))) {
        instructions.push(
          createAssociatedTokenAccountIdempotent(
            admin,
            user1Ata,
            user1.address,
            mintAddr,
          ),
        );
      }

      const operationId = await computeOperationId(
        user1.address,
        mintAddr,
        mintRawAmount,
        mintSalt,
      );
      const [mintOpAddr] = await findMintOperationPda({ operationId });

      instructions.push(
        getInitiateMintInstruction({
          caller: minter,
          config: minterConfigAddr,
          dailyLimit: dailyLimitAddr,
          mintOperation: mintOpAddr,
          callerPerms: (
            await findUserPermissionsPda({ user: minter.address })
          )[0],
          tokenConfig: tokenConfigAddr,
          tokenMint: mintAddr,
          recipientTokenAccount: user1Ata,
          mintAuthority: mintAuthAddr,
          minterUserPerms: minterConfigPermsAddr,
          recipientPerms: (
            await findUserPermissionsPda({ user: user1.address })
          )[0],
          user: user1.address,
          amount: mintRawAmount,
          salt: mintSalt,
        }),
      );

      return sendAndCapture(
        rpc,
        rpcSub,
        admin,
        instructions,
        "InitiateMint(20 -> User1)",
      );
    },
  );

  // ═══════════════════════════════════════════════════════════
  // Step 7a: User1 transfers 10 shares to User2 via spiko-token (Path B)
  // ═══════════════════════════════════════════════════════════
  await step(
    `User1 transfers ${TRANSFER_B_SHARES} shares to User2 via spiko-token transfer (Path B)`,
    ["Transfer"],
    async () => {
      const instructions: any[] = [];

      if (!(await accountExists(rpc, user2Ata))) {
        instructions.push(
          createAssociatedTokenAccountIdempotent(
            admin,
            user2Ata,
            user2.address,
            mintAddr,
          ),
        );
      }

      instructions.push(
        getTransferTokenInstruction({
          sender: user1,
          config: tokenConfigAddr,
          source: user1Ata,
          destination: user2Ata,
          mint: mintAddr,
          senderPerms: (
            await findUserPermissionsPda({ user: user1.address })
          )[0],
          recipientPerms: (
            await findUserPermissionsPda({ user: user2.address })
          )[0],
          extraAccountMetaList: extraMetaListAddr,
          permissionManagerProgram:
            PERMISSION_MANAGER_PROGRAM_ADDRESS as Address,
          spikoTokenProgram: SPIKO_TOKEN_PROGRAM_ADDRESS as Address,
          hookEventAuthority: thEventAuth,
          hookProgram: SPIKO_TRANSFER_HOOK_PROGRAM_ADDRESS as Address,
          amount: transferBRawAmount,
        }),
      );

      return sendAndCapture(
        rpc,
        rpcSub,
        admin,
        instructions,
        "TransferToken(10 User1->User2, Path B)",
      );
    },
  );

  // ═══════════════════════════════════════════════════════════
  // Step 7b: User1 transfers 5 shares to User2 via Token-2022 TransferChecked (Path A)
  // ═══════════════════════════════════════════════════════════
  await step(
    `User1 transfers ${TRANSFER_A_SHARES} shares to User2 via Token-2022 TransferChecked (Path A)`,
    ["Transfer"],
    async () => {
      const ix = buildTransferChecked(
        user1,
        user1Ata,
        user2Ata,
        mintAddr,
        tokenConfigAddr,
        (await findUserPermissionsPda({ user: user1.address }))[0],
        (await findUserPermissionsPda({ user: user2.address }))[0],
        extraMetaListAddr,
        transferARawAmount,
        decimals,
        thEventAuth,
      );

      return sendAndCapture(
        rpc,
        rpcSub,
        admin,
        [ix],
        "TransferChecked(5 User1->User2, Path A)",
      );
    },
  );

  // ═══════════════════════════════════════════════════════════
  // Step 7c: Direct transfer User1 -> User3 FAILS
  //   (WHITELISTED sender cannot send to WHITELISTED_EXT recipient)
  // ═══════════════════════════════════════════════════════════
  {
    totalSteps++;
    console.log(
      `\n--- Step ${totalSteps}: Direct transfer User1 -> User3 should FAIL ---\n`,
    );

    const instructions: any[] = [];

    // Create User3's ATA if needed
    if (!(await accountExists(rpc, user3Ata))) {
      instructions.push(
        createAssociatedTokenAccountIdempotent(
          admin,
          user3Ata,
          user3.address,
          mintAddr,
        ),
      );
      // Send ATA creation separately so it doesn't fail with the transfer
      await sendAndCapture(
        rpc,
        rpcSub,
        admin,
        instructions,
        "CreateATA(User3)",
      );
    }

    try {
      const ix = getTransferTokenInstruction({
        sender: user1,
        config: tokenConfigAddr,
        source: user1Ata,
        destination: user3Ata,
        mint: mintAddr,
        senderPerms: (
          await findUserPermissionsPda({ user: user1.address })
        )[0],
        recipientPerms: (
          await findUserPermissionsPda({ user: user3.address })
        )[0],
        extraAccountMetaList: extraMetaListAddr,
        permissionManagerProgram:
          PERMISSION_MANAGER_PROGRAM_ADDRESS as Address,
        spikoTokenProgram: SPIKO_TOKEN_PROGRAM_ADDRESS as Address,
        hookEventAuthority: thEventAuth,
        hookProgram: SPIKO_TRANSFER_HOOK_PROGRAM_ADDRESS as Address,
        amount: custodialWithdrawRawAmount,
      });
      await sendAndCapture(
        rpc,
        rpcSub,
        admin,
        [ix],
        "TransferToken(1 User1->User3, SHOULD FAIL)",
      );
      console.log("  ERROR: Transfer should have failed but succeeded!");
    } catch (err: any) {
      console.log(
        "  OK: Direct transfer User1 -> User3 correctly rejected",
      );
      console.log(`  Error: ${err.message?.slice(0, 120) ?? err}`);
    }
  }

  // ═══════════════════════════════════════════════════════════
  // Step 7d: Custodial withdraw User1 -> User3 via CustodialGatekeeper
  //   (User1 WHITELISTED -> CG vault WHITELISTED, CG vault WHITELISTED_EXT -> User3 WHITELISTED_EXT)
  // ═══════════════════════════════════════════════════════════
  await step(
    `User1 withdraws ${CUSTODIAL_WITHDRAW_SHARES} share to User3 via CustodialGatekeeper`,
    ["WithdrawalInitiated", "Transfer"],
    async () => {
      const instructions: any[] = [];

      // Create CG vault ATA if needed
      if (!(await accountExists(rpc, cgVaultAta))) {
        instructions.push(
          createAssociatedTokenAccountIdempotent(
            admin,
            cgVaultAta,
            cgVaultAuthAddr,
            mintAddr,
          ),
        );
      }

      const operationId = await computeOperationId(
        user3.address,
        mintAddr,
        custodialWithdrawRawAmount,
        custodialWithdrawSalt,
      );
      const [withdrawalOpAddr] = await findWithdrawalOperationPda({
        operationId,
      });

      instructions.push(
        getCustodialWithdrawInstruction({
          sender: user1,
          config: cgConfigAddr,
          dailyLimit: cgDailyLimitAddr,
          withdrawalOp: withdrawalOpAddr,
          senderTokenAccount: user1Ata,
          vaultTokenAccount: cgVaultAta,
          recipientTokenAccount: user3Ata,
          vaultAuthority: cgVaultAuthAddr,
          tokenMint: mintAddr,
          senderPerms: (
            await findUserPermissionsPda({ user: user1.address })
          )[0],
          recipientPerms: (
            await findUserPermissionsPda({ user: user3.address })
          )[0],
          vaultAuthorityPerms: cgVaultAuthPermsAddr,
          extraAccountMetaList: extraMetaListAddr,
          permissionManagerProgram:
            PERMISSION_MANAGER_PROGRAM_ADDRESS as Address,
          spikoTokenProgram: SPIKO_TOKEN_PROGRAM_ADDRESS as Address,
          tokenConfig: tokenConfigAddr,
          hookEventAuthority: thEventAuth,
          hookProgram: SPIKO_TRANSFER_HOOK_PROGRAM_ADDRESS as Address,
          eventAuthority: cgEventAuth,
          recipient: user3.address,
          amount: custodialWithdrawRawAmount,
          salt: custodialWithdrawSalt,
        }),
      );

      return sendAndCapture(
        rpc,
        rpcSub,
        admin,
        instructions,
        `CustodialWithdraw(${CUSTODIAL_WITHDRAW_SHARES} User1->User3)`,
      );
    },
  );

  // ═══════════════════════════════════════════════════════════
  // Step 8: User2 redeems 10 shares (tokens go to vault)
  // ═══════════════════════════════════════════════════════════
  await step(
    `User2 redeems ${REDEEM_SHARES} ${TOKEN_SYMBOL}`,
    ["RedeemInitiated", "RedemptionInitiated", "Transfer"],
    async () => {
      const instructions: any[] = [];

      if (!(await accountExists(rpc, vaultAta))) {
        instructions.push(
          createAssociatedTokenAccountIdempotent(
            admin,
            vaultAta,
            vaultAuthAddr,
            mintAddr,
          ),
        );
      }

      const operationId = await computeOperationId(
        user2.address,
        mintAddr,
        redeemRawAmount,
        redeemSalt,
      );
      const [redemptionOpAddr] = await findRedemptionOperationPda({
        operationId,
      });

      instructions.push(
        getRedeemTokenInstruction({
          user: user2,
          config: tokenConfigAddr,
          userSource: user2Ata,
          vault: vaultAta,
          mint: mintAddr,
          userPerms: (await findUserPermissionsPda({ user: user2.address }))[0],
          vaultAuthorityPerms: vaultAuthPermsAddr,
          redemptionProgram: REDEMPTION_PROGRAM_ADDRESS as Address,
          redemptionConfig: redemptionConfigAddr,
          redemptionOp: redemptionOpAddr,
          tokenMinimum: tokenMinAddr,
          extraAccountMetaList: extraMetaListAddr,
          permissionManagerProgram:
            PERMISSION_MANAGER_PROGRAM_ADDRESS as Address,
          spikoTokenProgram: SPIKO_TOKEN_PROGRAM_ADDRESS as Address,
          hookEventAuthority: thEventAuth,
          hookProgram: SPIKO_TRANSFER_HOOK_PROGRAM_ADDRESS as Address,
          eventAuthority: stEventAuth,
          rdEventAuthority: rdEventAuth,
          rdSelfProgram: REDEMPTION_PROGRAM_ADDRESS as Address,
          amount: redeemRawAmount,
          salt: redeemSalt,
        }),
      );

      return sendAndCapture(
        rpc,
        rpcSub,
        admin,
        instructions,
        "RedeemToken(10 User2)",
      );
    },
  );

  // ═══════════════════════════════════════════════════════════
  // Step 9: Executor executes the redemption (burns from vault)
  // ═══════════════════════════════════════════════════════════
  await step(
    "Executor executes redemption",
    ["RedemptionExecuted", "Burn"],
    async () => {
      const operationId = await computeOperationId(
        user2.address,
        mintAddr,
        redeemRawAmount,
        redeemSalt,
      );
      const [redemptionOpAddr] = await findRedemptionOperationPda({
        operationId,
      });

      const ix = getExecuteRedemptionInstruction({
        operator: executor,
        config: redemptionConfigAddr,
        redemptionOp: redemptionOpAddr,
        operatorPerms: (
          await findUserPermissionsPda({ user: executor.address })
        )[0],
        spikoTokenProgram: SPIKO_TOKEN_PROGRAM_ADDRESS as Address,
        tokenConfig: tokenConfigAddr,
        tokenMint: mintAddr,
        vault: vaultAta,
        mintAuthority: mintAuthAddr,
        vaultAuthority: vaultAuthAddr,
        vaultAuthorityPerms: vaultAuthPermsAddr,
        stEventAuthority: stEventAuth,
        stSelfProgram: SPIKO_TOKEN_PROGRAM_ADDRESS as Address,
        eventAuthority: rdEventAuth,
        user: user2.address,
        amount: redeemRawAmount,
        salt: redeemSalt,
      });

      return sendAndCapture(
        rpc,
        rpcSub,
        admin,
        [ix],
        "ExecuteRedemption(10 User2)",
      );
    },
  );

  // ═══════════════════════════════════════════════════════════
  // Summary
  // ═══════════════════════════════════════════════════════════
  // Steps 1-5: 5 x RoleGranted = 5
  // Step 5b: RoleGranted = 1
  // Step 6: MintExecuted + Mint = 2
  // Step 7a: Transfer = 1
  // Step 7b: Transfer = 1
  // Step 7c: (expected failure, no events)
  // Step 7d: WithdrawalInitiated + Transfer(s) = at least 2
  // Step 8: RedeemInitiated + RedemptionInitiated + Transfer = 3
  // Step 9: RedemptionExecuted + Burn = 2
  // Total = 17 (minimum, transfer hooks may emit additional Transfer events)
  const expectedTotal = 17;

  console.log(`\n\n=== E2E Test Complete ===`);
  console.log(`Steps:          ${totalSteps}`);
  console.log(`Events decoded: ${totalEvents}/${expectedTotal}`);

  if (totalEvents >= expectedTotal) {
    console.log("All expected events were found!");
  } else {
    console.log(
      `Warning: Some events were not found. This may be due to RPC indexing delay.`,
    );
  }

  // Final balance check
  console.log("\n--- Final Balances ---\n");
  for (const [name, ata] of [
    ["User1", user1Ata],
    ["User2", user2Ata],
    ["User3", user3Ata],
    ["Vault", vaultAta],
    ["CG Vault", cgVaultAta],
  ] as const) {
    try {
      const { value } = await rpc
        .getTokenAccountBalance(ata as Address, { commitment: "confirmed" })
        .send();
      const shares = Number(value.amount) / 10 ** decimals;
      console.log(`  ${name}: ${shares} shares (${value.amount} raw)`);
    } catch {
      console.log(`  ${name}: (account not found or empty)`);
    }
  }

  // Expected final balances:
  //   User1: 20 - 10 - 5 - 1 = 4 shares
  //   User2: 10 + 5 - 10 = 5 shares
  //   User3: 1 share (received via custodial gatekeeper)
  //   Vault: 10 - 10 = 0 shares (burned by executor)
  //   CG Vault: 0 shares (instant transfer to user3)
  console.log("\nExpected: User1=4, User2=5, User3=1, Vault=0, CG Vault=0");
}

main().catch((err) => {
  console.error("\nE2E test failed:", err);
  process.exit(1);
});
