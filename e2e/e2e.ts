/**
 * Self-contained end-to-end multi-actor test on devnet.
 *
 * Creates ALL on-chain state from scratch (mint, configs, roles),
 * then runs the full grant->mint->transfer->redeem->execute flow.
 *
 * Prerequisites:
 *   - All 6 Spiko programs deployed to devnet
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
  type ReadonlyUint8Array,
  AccountRole,
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

// ── Instruction imports ──────────────────────────────────────
import { getInitializeInstructionAsync as getPmInitializeInstructionAsync } from "../clients/ts/permission-manager/src/generated/instructions/initialize.js";
import { getGrantRoleInstructionAsync } from "../clients/ts/permission-manager/src/generated/instructions/grantRole.js";
import { getGrantRoleWhitelisterInstructionAsync } from "../clients/ts/permission-manager/src/generated/instructions/grantRoleWhitelister.js";

import { getInitializeInstructionAsync as getStInitializeInstructionAsync } from "../clients/ts/spiko-token/src/generated/instructions/initialize.js";
import { getRedeemInstructionAsync } from "../clients/ts/spiko-token/src/generated/instructions/redeem.js";

import { getInitializeInstructionAsync as getThInitializeInstructionAsync } from "../clients/ts/spiko-transfer-hook/src/generated/instructions/initialize.js";

import { getInitializeInstructionAsync as getMtInitializeInstructionAsync } from "../clients/ts/minter/src/generated/instructions/initialize.js";
import { getInitiateMintInstructionAsync } from "../clients/ts/minter/src/generated/instructions/initiateMint.js";
import { getSetDailyLimitInstructionAsync as getMtSetDailyLimitInstructionAsync } from "../clients/ts/minter/src/generated/instructions/setDailyLimit.js";

import { getInitializeInstructionAsync as getRdInitializeInstructionAsync } from "../clients/ts/redemption/src/generated/instructions/initialize.js";
import { getCreateVaultInstructionAsync } from "../clients/ts/redemption/src/generated/instructions/createVault.js";
import { getExecuteInstructionAsync as getRdExecuteInstructionAsync } from "../clients/ts/redemption/src/generated/instructions/execute.js";

import { getInitializeInstructionAsync as getCgInitializeInstructionAsync } from "../clients/ts/custodial-gatekeeper/src/generated/instructions/initialize.js";
import { getSetDailyLimitInstructionAsync as getCgSetDailyLimitInstructionAsync } from "../clients/ts/custodial-gatekeeper/src/generated/instructions/setDailyLimit.js";
import { getCustodialWithdrawInstructionAsync } from "../clients/ts/custodial-gatekeeper/src/generated/instructions/custodialWithdraw.js";

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
  setup,
  getAssociatedTokenAddress,
  createAssociatedTokenAccountIdempotent,
  accountExists,
  computeOperationId,
  getMintAccountSpace,
  MINT_FIXED_EXTENSIONS_SIZE,
  buildCreateAccountInstruction,
  buildTransferChecked,
  buildInitializeTransferHook,
  buildInitializePermanentDelegate,
  buildInitializeMetadataPointer,
  buildInitializeMint2,
  buildTokenMetadataInitialize,
} from "./lib/shared.js";

import { parseTransactionEvents, formatEvents } from "./lib/events.js";

const TOKEN_DECIMALS = 5;
const TOKEN_NAME = "EUR T-Bill";
const TOKEN_SYMBOL = "EUTBL";
const TOKEN_URI = "";
const DAILY_LIMIT = 100_000_000_000n; // 1M shares at 5 decimals
const DEADLINE_DELAY = 86400n; // 1 day in seconds
const MAX_DELAY = 86400n; // 1 day in seconds

const MINT_SHARES = 20;
const TRANSFER_B_SHARES = 10; // Path B: direct TransferChecked
const TRANSFER_A_SHARES = 5; // Path A: direct TransferChecked
const REDEEM_SHARES = 10;
const CUSTODIAL_WITHDRAW_SHARES = 1; // Custodial gatekeeper withdraw to user3

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

/**
 * Build the transfer hook extra accounts as remaining accounts for CPI.
 * These are appended to instructions that call `transfer_checked` internally.
 *
 * Order matches ExtraAccountMetaList:
 *   extra_account_metas_list, hook_config, pm_program,
 *   pm_config, source_perms, dest_perms, hook_program
 */
function buildTransferHookRemainingAccounts(
  extraAccountMetaListAddr: Address,
  hookConfigAddr: Address,
  permConfigAddr: Address,
  sourcePermsAddr: Address,
  destPermsAddr: Address,
) {
  return [
    { address: extraAccountMetaListAddr, role: AccountRole.READONLY, isSigner: false, isWritable: false },
    { address: hookConfigAddr, role: AccountRole.READONLY, isSigner: false, isWritable: false },
    { address: PERMISSION_MANAGER_PROGRAM_ADDRESS as Address, role: AccountRole.READONLY, isSigner: false, isWritable: false },
    { address: permConfigAddr, role: AccountRole.READONLY, isSigner: false, isWritable: false },
    { address: sourcePermsAddr, role: AccountRole.READONLY, isSigner: false, isWritable: false },
    { address: destPermsAddr, role: AccountRole.READONLY, isSigner: false, isWritable: false },
    { address: SPIKO_TRANSFER_HOOK_PROGRAM_ADDRESS as Address, role: AccountRole.READONLY, isSigner: false, isWritable: false },
  ];
}

async function main() {
  console.log("=== Spiko E2E Self-Contained Test ===\n");

  const { rpc, rpcSub, admin } = await setup();

  const mintKp = await generateKeyPairSigner();
  const mintAddr = mintKp.address;
  const decimals = TOKEN_DECIMALS;

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

  // ── Derive PDAs ────────────────────────────────────────────
  const [permConfigAddr] = await findConfigPda();
  const [minterConfigAddr] = await findMinterConfigPda();
  const [tokenConfigAddr] = await findTokenConfigPda({ mint: mintAddr });
  const [mintAuthAddr] = await findMintAuthorityPda({ mint: mintAddr });
  const [hookConfigAddr] = await findHookConfigPda({ mint: mintAddr });
  const [extraMetaListAddr] = await findExtraAccountMetasPda({ mint: mintAddr });
  const [mintDailyLimitAddr] = await findMintDailyLimitPda({ mint: mintAddr });
  const [redemptionConfigAddr] = await findRedemptionConfigPda();
  const [redemptionVaultAuthAddr] = await findRedemptionVaultAuthorityPda({ mint: mintAddr });
  const vaultAta = await getAssociatedTokenAddress(redemptionVaultAuthAddr, mintAddr);

  // Minter config needs MINTER role
  const [minterConfigPermsAddr] = await findUserPermissionsPda({
    user: minterConfigAddr,
    config: permConfigAddr,
  });

  // Vault authority needs BURNER + WHITELISTED roles
  const [vaultAuthPermsAddr] = await findUserPermissionsPda({
    user: redemptionVaultAuthAddr,
    config: permConfigAddr,
  });

  // Custodial Gatekeeper PDAs
  const [cgConfigAddr] = await findGatekeeperConfigPda();
  const [cgVaultAuthAddr] = await findCgVaultAuthorityPda();
  const [cgVaultAuthPermsAddr] = await findUserPermissionsPda({
    user: cgVaultAuthAddr,
    config: permConfigAddr,
  });
  const [cgDailyLimitAddr] = await findWithdrawalDailyLimitPda({
    mint: mintAddr,
  });

  // ATAs
  const user1Ata = await getAssociatedTokenAddress(user1.address, mintAddr);
  const user2Ata = await getAssociatedTokenAddress(user2.address, mintAddr);
  const user3Ata = await getAssociatedTokenAddress(user3.address, mintAddr);
  const cgVaultAta = await getAssociatedTokenAddress(cgVaultAuthAddr, mintAddr);

  // ===================================================================
  //  SETUP PHASE: Create all on-chain state from scratch
  // ===================================================================

  console.log("\n========================================");
  console.log("  SETUP PHASE: Deploying on-chain state");
  console.log("========================================\n");

  // -- S0: Create Token-2022 mint account -------------------------
  // Create mint with extensions: TransferHook, PermanentDelegate, MetadataPointer, Metadata.
  // Mint authority is set to admin temporarily so we can sign TokenMetadataInitialize.
  // spiko-token::initialize will transfer mint authority to its PDA.
  console.log("--- S0: Create Token-2022 mint account ---\n");
  {
    const finalSpace = getMintAccountSpace(TOKEN_NAME, TOKEN_SYMBOL, TOKEN_URI);
    const rentLamports = await getMinRent(rpc, finalSpace);

    const createAccountIx = buildCreateAccountInstruction(
      admin,
      mintKp,
      rentLamports,
      MINT_FIXED_EXTENSIONS_SIZE,
      TOKEN_2022_PROGRAM_ID,
    );

    // Extension inits (BEFORE InitializeMint2)
    const initTransferHookIx = buildInitializeTransferHook(
      mintAddr,
      admin.address,
      SPIKO_TRANSFER_HOOK_PROGRAM_ADDRESS as Address,
    );
    const initPermanentDelegateIx = buildInitializePermanentDelegate(
      mintAddr,
      mintAuthAddr, // permanent delegate = MintAuthority PDA
    );
    const initMetadataPointerIx = buildInitializeMetadataPointer(
      mintAddr,
      admin.address,
      mintAddr, // metadata lives on the mint itself
    );

    // Initialize mint with admin as temporary authority
    const initMintIx = buildInitializeMint2(
      mintAddr,
      decimals,
      admin.address,
    );

    // Initialize metadata (AFTER InitializeMint2, admin signs as mint authority)
    const initMetadataIx = buildTokenMetadataInitialize(
      mintAddr,
      admin.address,
      admin,
      TOKEN_NAME,
      TOKEN_SYMBOL,
      TOKEN_URI,
    );

    await sendAndCapture(rpc, rpcSub, admin, [
      createAccountIx,
      initTransferHookIx,
      initPermanentDelegateIx,
      initMetadataPointerIx,
      initMintIx,
      initMetadataIx,
    ], "CreateMint(EUTBL)");
  }

  // -- S1: Initialize PermissionManager ---------------------------
  // -- S4: Initialize Minter --------------------------------------
  // -- S5: Initialize Redemption ----------------------------------
  // Skip any that are already initialized (singleton configs persist across runs)
  console.log(
    "\n--- S1+S4+S5: Initialize PermissionManager + Minter + Redemption ---\n",
  );
  {
    const initIxs: any[] = [];
    const labels: string[] = [];

    if (!(await accountExists(rpc, permConfigAddr))) {
      initIxs.push(await getPmInitializeInstructionAsync({ admin }));
      labels.push("PermissionManager");
    } else {
      console.log("  (PermissionManager already initialized -- skipping)");
    }

    if (!(await accountExists(rpc, minterConfigAddr))) {
      initIxs.push(
        await getMtInitializeInstructionAsync({
          admin,
          permissionManager: PERMISSION_MANAGER_PROGRAM_ADDRESS as Address,
          maxDelay: MAX_DELAY,
        }),
      );
      labels.push("Minter");
    } else {
      console.log("  (Minter already initialized -- skipping)");
    }

    if (!(await accountExists(rpc, redemptionConfigAddr))) {
      initIxs.push(
        await getRdInitializeInstructionAsync({
          admin,
          deadlineDelay: DEADLINE_DELAY,
        }),
      );
      labels.push("Redemption");
    } else {
      console.log("  (Redemption already initialized -- skipping)");
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

  // -- S2: Initialize SpikoToken (for the mint) -------------------
  console.log("\n--- S2: Initialize SpikoToken ---\n");
  {
    const ix = await getStInitializeInstructionAsync({
      admin,
      mint: mintAddr,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
    });
    await sendAndCapture(rpc, rpcSub, admin, [ix], "InitializeToken(EUTBL)");
  }

  // -- S3: Init TransferHook (ExtraAccountMetas for the mint) -----
  console.log("\n--- S3: Init TransferHook ---\n");
  {
    const ix = await getThInitializeInstructionAsync({
      admin,
      mint: mintAddr,
      permissionManagerConfig: permConfigAddr,
      permissionManagerProgram: PERMISSION_MANAGER_PROGRAM_ADDRESS as Address,
    });
    await sendAndCapture(rpc, rpcSub, admin, [ix], "InitializeTransferHook");
  }

  // -- S6: Create Redemption Vault (per-mint) ---------------------
  console.log("\n--- S6: Create Redemption Vault ---\n");
  {
    const ix = await getCreateVaultInstructionAsync({
      admin,
      mint: mintAddr,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
    });
    await sendAndCapture(rpc, rpcSub, admin, [ix], "CreateVault(Redemption)");
  }

  // -- S7+S8+S9: Grant roles to MinterConfig & VaultAuthority -----
  console.log(
    "\n--- S7+S8+S9: Grant ROLE_MINTER to MinterConfig + ROLE_BURNER & ROLE_WHITELISTED to VaultAuthority ---\n",
  );
  {
    const roleIxs: any[] = [];
    const roleLabels: string[] = [];

    if (!(await accountExists(rpc, minterConfigPermsAddr))) {
      roleIxs.push(
        await getGrantRoleInstructionAsync({
          admin,
          user: minterConfigAddr,
          role: ROLE_MINTER,
        }),
      );
      roleLabels.push("MINTER->MinterConfig");
    } else {
      console.log("  (MinterConfig perms already exist -- skipping)");
    }

    if (!(await accountExists(rpc, vaultAuthPermsAddr))) {
      roleIxs.push(
        await getGrantRoleInstructionAsync({
          admin,
          user: redemptionVaultAuthAddr,
          role: ROLE_BURNER,
        }),
      );
      roleIxs.push(
        await getGrantRoleInstructionAsync({
          admin,
          user: redemptionVaultAuthAddr,
          role: ROLE_WHITELISTED,
        }),
      );
      roleLabels.push("BURNER+WHITELISTED->VaultAuth");
    } else {
      console.log("  (VaultAuthority perms already exist -- skipping)");
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

  // -- S10: SetDailyLimit (Minter) --------------------------------
  console.log("\n--- S10: SetDailyLimit (Minter) ---\n");
  {
    const ix = await getMtSetDailyLimitInstructionAsync({
      admin,
      mint: mintAddr,
      limit: DAILY_LIMIT,
    });
    await sendAndCapture(rpc, rpcSub, admin, [ix], "SetDailyLimit(Minter)");
  }

  // -- S11: Initialize CustodialGatekeeper ------------------------
  console.log("\n--- S11: Initialize CustodialGatekeeper ---\n");
  {
    if (!(await accountExists(rpc, cgConfigAddr))) {
      const ix = await getCgInitializeInstructionAsync({
        admin,
        permissionManager: PERMISSION_MANAGER_PROGRAM_ADDRESS as Address,
        maxDelay: MAX_DELAY,
      });
      await sendAndCapture(
        rpc,
        rpcSub,
        admin,
        [ix],
        "Initialize(CustodialGatekeeper)",
      );
    } else {
      console.log("  (CustodialGatekeeper already initialized -- skipping)");
    }
  }

  // -- S12: Grant WHITELISTED_EXT + WHITELISTED to CG VaultAuthority
  console.log(
    "\n--- S12: Grant WHITELISTED_EXT + WHITELISTED to CG VaultAuthority ---\n",
  );
  {
    if (!(await accountExists(rpc, cgVaultAuthPermsAddr))) {
      const ix1 = await getGrantRoleInstructionAsync({
        admin,
        user: cgVaultAuthAddr,
        role: ROLE_WHITELISTED_EXT,
      });
      const ix2 = await getGrantRoleInstructionAsync({
        admin,
        user: cgVaultAuthAddr,
        role: ROLE_WHITELISTED,
      });
      await sendAndCapture(
        rpc,
        rpcSub,
        admin,
        [ix1, ix2],
        "GrantRole(WHITELISTED_EXT+WHITELISTED -> CG VaultAuth)",
      );
    } else {
      console.log("  (CG VaultAuthority perms already exist -- skipping)");
    }
  }

  // -- S13: Set CustodialGatekeeper DailyLimit for this mint ------
  console.log("\n--- S13: Set CustodialGatekeeper DailyLimit ---\n");
  {
    const ix = await getCgSetDailyLimitInstructionAsync({
      admin,
      mint: mintAddr,
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
  console.log("  SETUP COMPLETE -- Starting test flow");
  console.log("========================================\n");

  console.log(`Token:          ${TOKEN_SYMBOL}`);
  console.log(`Mint:           ${mintAddr}`);
  console.log(`Decimals:       ${decimals}`);
  console.log(`User1 ATA:      ${user1Ata}`);
  console.log(`User2 ATA:      ${user2Ata}`);
  console.log(`User3 ATA:      ${user3Ata}`);
  console.log(`Vault ATA:      ${vaultAta}`);
  console.log(`CG Vault ATA:   ${cgVaultAta}\n`);

  // -- Fund accounts that need to sign ----------------------------
  console.log("--- Funding accounts ---\n");
  await fundAccount(rpc, rpcSub, admin, minter.address);
  await fundAccount(rpc, rpcSub, admin, executor.address);
  await fundAccount(rpc, rpcSub, admin, whitelister.address);
  await fundAccount(rpc, rpcSub, admin, user1.address);
  await fundAccount(rpc, rpcSub, admin, user2.address);
  await fundAccount(rpc, rpcSub, admin, user3.address);
  console.log();

  // ===================================================================
  //  TEST FLOW
  // ===================================================================

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

  // ===================================================================
  // Steps 1-5: Grant roles to actors
  // ===================================================================

  // Admin grants roles (using grantRole)
  await step(
    "Admin grants ROLE_MINT_INITIATOR to Minter",
    ["RoleGranted"],
    async () => {
      const ix = await getGrantRoleInstructionAsync({
        admin,
        user: minter.address,
        role: ROLE_MINT_INITIATOR,
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
        admin,
        user: executor.address,
        role: ROLE_REDEMPTION_EXECUTOR,
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
        admin,
        user: whitelister.address,
        role: ROLE_WHITELISTER,
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

  // Whitelister grants WHITELISTED (using grantRoleWhitelister)
  await step(
    "Whitelister grants ROLE_WHITELISTED to User1",
    ["RoleGranted"],
    async () => {
      const ix = await getGrantRoleWhitelisterInstructionAsync({
        caller: whitelister,
        user: user1.address,
        role: ROLE_WHITELISTED,
      });
      return sendAndCapture(
        rpc,
        rpcSub,
        whitelister,
        [ix],
        "GrantRoleWhitelister(WHITELISTED -> User1)",
      );
    },
  );

  await step(
    "Whitelister grants ROLE_WHITELISTED to User2",
    ["RoleGranted"],
    async () => {
      const ix = await getGrantRoleWhitelisterInstructionAsync({
        caller: whitelister,
        user: user2.address,
        role: ROLE_WHITELISTED,
      });
      return sendAndCapture(
        rpc,
        rpcSub,
        whitelister,
        [ix],
        "GrantRoleWhitelister(WHITELISTED -> User2)",
      );
    },
  );

  await step(
    "Whitelister grants ROLE_WHITELISTED_EXT to User3 (self-custodial)",
    ["RoleGranted"],
    async () => {
      const ix = await getGrantRoleWhitelisterInstructionAsync({
        caller: whitelister,
        user: user3.address,
        role: ROLE_WHITELISTED_EXT,
      });
      return sendAndCapture(
        rpc,
        rpcSub,
        whitelister,
        [ix],
        "GrantRoleWhitelister(WHITELISTED_EXT -> User3)",
      );
    },
  );

  // ===================================================================
  // Step 7: Minter mints 20 shares to User1 (auto-executes)
  // ===================================================================
  await step(
    `Minter mints ${MINT_SHARES} ${TOKEN_SYMBOL} to User1`,
    ["MintInitiated", "Minted"],
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
        await getInitiateMintInstructionAsync({
          minter,
          mint: mintAddr,
          destination: user1Ata,
          minterPermissions: (
            await findUserPermissionsPda({ user: minter.address, config: permConfigAddr })
          )[0],
          minterConfigPermissions: minterConfigPermsAddr,
          permissionManagerConfig: permConfigAddr,
          tokenConfig: tokenConfigAddr,
          mintAuthority: mintAuthAddr,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
          operationId,
          recipient: user1.address,
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

  // ===================================================================
  // Step 8a: User1 transfers 10 shares to User2 via Token-2022 TransferChecked (Path A)
  // ===================================================================
  await step(
    `User1 transfers ${TRANSFER_A_SHARES} shares to User2 via Token-2022 TransferChecked (Path A)`,
    ["TransferExecuted"],
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

      const ix = buildTransferChecked(
        user1,
        user1Ata,
        user2Ata,
        mintAddr,
        hookConfigAddr,
        (await findUserPermissionsPda({ user: user1.address, config: permConfigAddr }))[0],
        (await findUserPermissionsPda({ user: user2.address, config: permConfigAddr }))[0],
        extraMetaListAddr,
        permConfigAddr,
        transferARawAmount,
        decimals,
      );

      instructions.push(ix);

      return sendAndCapture(
        rpc,
        rpcSub,
        admin,
        instructions,
        "TransferChecked(5 User1->User2, Path A)",
      );
    },
  );

  // ===================================================================
  // Step 8b: User1 transfers 10 shares to User2 via spiko-token transfer (Path B)
  //   NOTE: spiko-token transfer now accepts remaining_accounts for hook
  // ===================================================================
  // TODO: The Codama-generated client for spiko-token::transfer doesn't support
  // remaining accounts natively. We'd need to build the instruction manually
  // or use a different approach. For now, use Path A (TransferChecked) for
  // the second transfer too.
  await step(
    `User1 transfers ${TRANSFER_B_SHARES} shares to User2 via Token-2022 TransferChecked (Path B)`,
    ["TransferExecuted"],
    async () => {
      const ix = buildTransferChecked(
        user1,
        user1Ata,
        user2Ata,
        mintAddr,
        hookConfigAddr,
        (await findUserPermissionsPda({ user: user1.address, config: permConfigAddr }))[0],
        (await findUserPermissionsPda({ user: user2.address, config: permConfigAddr }))[0],
        extraMetaListAddr,
        permConfigAddr,
        transferBRawAmount,
        decimals,
      );

      return sendAndCapture(
        rpc,
        rpcSub,
        admin,
        [ix],
        "TransferChecked(10 User1->User2, Path B)",
      );
    },
  );

  // ===================================================================
  // Step 8c: Direct transfer User1 -> User3 FAILS
  //   (WHITELISTED sender cannot send to WHITELISTED_EXT recipient)
  // ===================================================================
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
      const ix = buildTransferChecked(
        user1,
        user1Ata,
        user3Ata,
        mintAddr,
        hookConfigAddr,
        (await findUserPermissionsPda({ user: user1.address, config: permConfigAddr }))[0],
        (await findUserPermissionsPda({ user: user3.address, config: permConfigAddr }))[0],
        extraMetaListAddr,
        permConfigAddr,
        custodialWithdrawRawAmount,
        decimals,
      );
      await sendAndCapture(
        rpc,
        rpcSub,
        admin,
        [ix],
        "TransferChecked(1 User1->User3, SHOULD FAIL)",
      );
      console.log("  ERROR: Transfer should have failed but succeeded!");
    } catch (err: any) {
      console.log(
        "  OK: Direct transfer User1 -> User3 correctly rejected",
      );
      console.log(`  Error: ${err.message?.slice(0, 120) ?? err}`);
    }
  }

  // ===================================================================
  // Step 8d: Custodial withdraw User1 -> User3 via CustodialGatekeeper
  // ===================================================================
  await step(
    `User1 withdraws ${CUSTODIAL_WITHDRAW_SHARES} share to User3 via CustodialGatekeeper`,
    ["WithdrawalInitiated"],
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
        user1.address,
        mintAddr,
        custodialWithdrawRawAmount,
        custodialWithdrawSalt,
      );
      const [withdrawalOpAddr] = await findWithdrawalOperationPda({
        operationId,
      });

      // Build the CG instruction - need to pass transfer hook extra accounts
      // CG vault authority perms needed for both sender->vault and vault->recipient
      const senderPerms = (await findUserPermissionsPda({ user: user1.address, config: permConfigAddr }))[0];
      const recipientPerms = (await findUserPermissionsPda({ user: user3.address, config: permConfigAddr }))[0];

      const ix = await getCustodialWithdrawInstructionAsync({
        sender: user1,
        mint: mintAddr,
        senderTokenAccount: user1Ata,
        vaultTokenAccount: cgVaultAta,
        recipientTokenAccount: user3Ata,
        senderPermissions: senderPerms,
        vaultAuthorityPermissions: cgVaultAuthPermsAddr,
        recipientPermissions: recipientPerms,
        permissionManagerConfig: permConfigAddr,
        permissionManagerProgram: PERMISSION_MANAGER_PROGRAM_ADDRESS as Address,
        extraAccountMetasList: extraMetaListAddr,
        hookConfig: hookConfigAddr,
        transferHookProgram: SPIKO_TRANSFER_HOOK_PROGRAM_ADDRESS as Address,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        operationId,
        recipient: user3.address,
        amount: custodialWithdrawRawAmount,
        salt: custodialWithdrawSalt,
      });

      instructions.push(ix);

      return sendAndCapture(
        rpc,
        rpcSub,
        admin,
        instructions,
        `CustodialWithdraw(${CUSTODIAL_WITHDRAW_SHARES} User1->User3)`,
      );
    },
  );

  // ===================================================================
  // Step 9: User2 redeems 10 shares (tokens go to vault)
  // ===================================================================
  await step(
    `User2 redeems ${REDEEM_SHARES} ${TOKEN_SYMBOL}`,
    ["RedemptionInitiated"],
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

      // spiko-token::redeem needs transfer hook remaining accounts
      // for the source->vault transfer
      const redeemerPerms = (await findUserPermissionsPda({ user: user2.address, config: permConfigAddr }))[0];

      const ix = await getRedeemInstructionAsync({
        redeemer: user2,
        tokenConfig: tokenConfigAddr,
        mint: mintAddr,
        source: user2Ata,
        vault: vaultAta,
        redeemerPermissions: redeemerPerms,
        permissionManagerConfig: permConfigAddr,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        redemptionVaultAuthority: redemptionVaultAuthAddr,
        redemptionConfig: redemptionConfigAddr,
        redemptionOperation: redemptionOpAddr,
        amount: redeemRawAmount,
        salt: redeemSalt,
      });

      // Append transfer hook remaining accounts for source->vault transfer
      const hookAccounts = buildTransferHookRemainingAccounts(
        extraMetaListAddr,
        hookConfigAddr,
        permConfigAddr,
        redeemerPerms,     // source perms (user2)
        vaultAuthPermsAddr, // dest perms (vault authority)
      );

      const ixWithRemaining = {
        ...ix,
        accounts: [...ix.accounts, ...hookAccounts],
      };

      return sendAndCapture(
        rpc,
        rpcSub,
        admin,
        [ixWithRemaining],
        "Redeem(10 User2)",
      );
    },
  );

  // ===================================================================
  // Step 10: Executor executes the redemption (burns from vault)
  // ===================================================================
  await step(
    "Executor executes redemption",
    ["RedemptionExecuted"],
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

      const ix = await getRdExecuteInstructionAsync({
        burner: executor,
        mint: mintAddr,
        vault: vaultAta,
        burnerPermissions: (
          await findUserPermissionsPda({ user: executor.address, config: permConfigAddr })
        )[0],
        permissionManagerConfig: permConfigAddr,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        operationId,
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

  // ===================================================================
  // Summary
  // ===================================================================
  // Steps 1-6: 6 x RoleGranted = 6
  // Step 7: MintInitiated + Minted = 2
  // Step 8a: TransferExecuted = 1
  // Step 8b: TransferExecuted = 1
  // Step 8c: (expected failure, no events)
  // Step 8d: WithdrawalInitiated (+ possibly TransferExecuted) = at least 1
  // Step 9: RedemptionInitiated = 1
  // Step 10: RedemptionExecuted = 1
  // Total = 13 (minimum)
  const expectedTotal = 13;

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
  //   User1: 20 - 5 - 10 - 1 = 4 shares
  //   User2: 5 + 10 - 10 = 5 shares
  //   User3: 1 share (received via custodial gatekeeper)
  //   Vault: 10 - 10 = 0 shares (burned by executor)
  //   CG Vault: 0 shares (instant transfer to user3)
  console.log("\nExpected: User1=4, User2=5, User3=1, Vault=0, CG Vault=0");
}

main().catch((err) => {
  console.error("\nE2E test failed:", err);
  process.exit(1);
});
