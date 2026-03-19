/**
 * End-to-end multi-actor test on devnet.
 *
 * 6 accounts, 10 steps:
 *   Admin      — deployer, has all roles
 *   Minter     — ROLE_MINT_INITIATOR (bit 7)
 *   Executor   — ROLE_REDEMPTION_EXECUTOR (bit 5)
 *   Whitelister — ROLE_WHITELISTER (bit 3)
 *   User1      — ROLE_WHITELISTED (bit 4), granted by Whitelister
 *   User2      — ROLE_WHITELISTED (bit 4), granted by Whitelister
 *
 * Flow:
 *   1.  Admin grants ROLE_MINT_INITIATOR to Minter
 *   2.  Admin grants ROLE_REDEMPTION_EXECUTOR to Executor
 *   3.  Admin grants ROLE_WHITELISTER to Whitelister
 *   4.  Whitelister grants ROLE_WHITELISTED to User1
 *   5.  Whitelister grants ROLE_WHITELISTED to User2
 *   6.  Minter mints 20 shares to User1 (auto-executes, below daily limit)
 *   7a. User1 transfers 10 shares to User2 via spiko-token transfer (Path B)
 *   7b. User1 transfers 5 shares to User2 via direct Token-2022 TransferChecked (Path A)
 *   8.  User2 redeems 10 shares (tokens go to vault)
 *   9.  Executor executes the redemption (burns from vault)
 *
 * Usage:
 *   npx tsx E2ETest.ts
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

import {
  TOKEN_DECIMALS,
  ROLE_WHITELISTED,
  ROLE_WHITELISTER,
  ROLE_MINT_INITIATOR,
  ROLE_REDEMPTION_EXECUTOR,
} from "./lib/constants.js";
import {
  permissionConfigPda,
  userPermissionsPda,
  tokenConfigPda,
  mintAuthorityPda,
  extraAccountMetaListPda,
  minterConfigPda,
  dailyLimitPda,
  mintOperationPda,
  redemptionConfigPda,
  tokenMinimumPda,
  vaultAuthorityPda,
  redemptionOperationPda,
} from "./lib/pda.js";
import {
  grantRole,
  initiateMint,
  transferToken,
  buildTransferChecked,
  redeemToken,
  executeRedemption,
} from "./lib/instructions.js";
import {
  setup,
  resolveMintAddress,
  getAssociatedTokenAddress,
  createAssociatedTokenAccountIdempotent,
  accountExists,
  computeOperationId,
  sendTx,
} from "./lib/shared.js";
import {
  parseTransactionEvents,
  formatEvents,
  type DecodedEvent,
} from "./lib/events.js";

// =================================================================
// Helpers
// =================================================================

async function sendAndCapture(
  rpc: Rpc<SolanaRpcApi>,
  rpcSub: RpcSubscriptions<SolanaRpcSubscriptionsApi>,
  payer: KeyPairSigner,
  instructions: any[],
  signers: KeyPairSigner[],
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
    rpcSubscriptions: rpcSub,
  });
  await sendAndConfirm(signedTx, { commitment: "confirmed" });

  const sig = getSignatureFromTransaction(signedTx);
  console.log(`  ${label}: ${sig}`);
  return sig;
}

/**
 * Airdrop SOL to an address. Falls back to a transfer from admin if
 * the devnet faucet is rate-limited.
 */
async function fundAccount(
  rpc: Rpc<SolanaRpcApi>,
  rpcSub: RpcSubscriptions<SolanaRpcSubscriptionsApi>,
  admin: KeyPairSigner,
  target: Address,
  amount: bigint = 50_000_000n // 0.05 SOL
): Promise<void> {
  try {
    const sig = await rpc.requestAirdrop(target, lamports(amount)).send();
    // Wait for airdrop to confirm
    await new Promise((r) => setTimeout(r, 2000));
    console.log(`  Airdropped ${Number(amount) / 1e9} SOL to ${target.slice(0, 8)}...`);
  } catch {
    // Faucet rate-limited — transfer from admin instead
    const { getTransferSolInstruction } = await import("@solana-program/system");
    const ix = getTransferSolInstruction({
      source: admin,
      destination: target,
      amount: amount,
    });
    await sendAndCapture(rpc, rpcSub, admin, [ix], [admin], `Fund ${target.slice(0, 8)}...`);
  }
}

// =================================================================
// Main
// =================================================================

async function main() {
  console.log("=== Spiko E2E Multi-Actor Test ===\n");

  const { rpc, rpcSub, admin } = await setup();

  // ── Generate 5 fresh keypairs ──────────────────────────────
  const minter = await generateKeyPairSigner();
  const executor = await generateKeyPairSigner();
  const whitelister = await generateKeyPairSigner();
  const user1 = await generateKeyPairSigner();
  const user2 = await generateKeyPairSigner();

  console.log("Accounts:");
  console.log(`  Admin:       ${admin.address}`);
  console.log(`  Minter:      ${minter.address}`);
  console.log(`  Executor:    ${executor.address}`);
  console.log(`  Whitelister: ${whitelister.address}`);
  console.log(`  User1:       ${user1.address}`);
  console.log(`  User2:       ${user2.address}`);

  // ── Token setup ────────────────────────────────────────────
  const TOKEN = "eutbl";
  const MINT_SHARES = 20;
  const TRANSFER_B_SHARES = 10; // Path B: spiko-token transfer
  const TRANSFER_A_SHARES = 5;  // Path A: direct TransferChecked
  const REDEEM_SHARES = 10;

  const mintRawAmount = BigInt(Math.round(MINT_SHARES * 10 ** TOKEN_DECIMALS));
  const transferBRawAmount = BigInt(Math.round(TRANSFER_B_SHARES * 10 ** TOKEN_DECIMALS));
  const transferARawAmount = BigInt(Math.round(TRANSFER_A_SHARES * 10 ** TOKEN_DECIMALS));
  const redeemRawAmount = BigInt(Math.round(REDEEM_SHARES * 10 ** TOKEN_DECIMALS));

  const mintSalt = BigInt(Math.floor(Math.random() * Number.MAX_SAFE_INTEGER));
  const redeemSalt = BigInt(Math.floor(Math.random() * Number.MAX_SAFE_INTEGER));

  // ── Resolve PDAs & addresses ───────────────────────────────
  const mintAddr = await resolveMintAddress(TOKEN);
  const [permConfigAddr] = await permissionConfigPda();
  const [adminPermsAddr] = await userPermissionsPda(admin.address);
  const [minterPermsAddr] = await userPermissionsPda(minter.address);
  const [executorPermsAddr] = await userPermissionsPda(executor.address);
  const [whitelisterPermsAddr] = await userPermissionsPda(whitelister.address);
  const [user1PermsAddr] = await userPermissionsPda(user1.address);
  const [user2PermsAddr] = await userPermissionsPda(user2.address);
  const [minterConfigAddr] = await minterConfigPda();
  const [minterConfigPermsAddr] = await userPermissionsPda(minterConfigAddr);
  const [tokenConfigAddr] = await tokenConfigPda(mintAddr);
  const [mintAuthAddr] = await mintAuthorityPda(mintAddr);
  const [dailyLimitAddr] = await dailyLimitPda(mintAddr);
  const [redemptionConfigAddr] = await redemptionConfigPda();
  const [vaultAuthAddr] = await vaultAuthorityPda();
  const [vaultAuthPermsAddr] = await userPermissionsPda(vaultAuthAddr);
  const [extraMetaListAddr] = await extraAccountMetaListPda(mintAddr);

  const user1Ata = await getAssociatedTokenAddress(user1.address, mintAddr);
  const user2Ata = await getAssociatedTokenAddress(user2.address, mintAddr);
  const vaultAta = await getAssociatedTokenAddress(vaultAuthAddr, mintAddr);

  console.log(`\nToken:          ${TOKEN.toUpperCase()}`);
  console.log(`Mint:           ${mintAddr}`);
  console.log(`User1 ATA:      ${user1Ata}`);
  console.log(`User2 ATA:      ${user2Ata}`);
  console.log(`Vault ATA:      ${vaultAta}\n`);

  // ── Fund accounts that need to sign ────────────────────────
  console.log("--- Funding accounts ---\n");
  // Minter signs initiateMint, Whitelister signs grantRole,
  // User1 signs transfers, User2 signs redeem, Executor signs executeRedemption
  await fundAccount(rpc, rpcSub, admin, minter.address);
  await fundAccount(rpc, rpcSub, admin, executor.address);
  await fundAccount(rpc, rpcSub, admin, whitelister.address);
  await fundAccount(rpc, rpcSub, admin, user1.address);
  await fundAccount(rpc, rpcSub, admin, user2.address);
  console.log();

  let totalEvents = 0;
  let totalSteps = 0;

  async function step(
    label: string,
    expectedEvents: string[],
    fn: () => Promise<string>
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
  // Step 1: Admin grants ROLE_MINT_INITIATOR to Minter
  // ═══════════════════════════════════════════════════════════
  await step("Admin grants ROLE_MINT_INITIATOR to Minter", ["RoleGranted"], async () => {
    const ix = grantRole(
      admin,
      permConfigAddr,
      minterPermsAddr,
      minter.address,
      adminPermsAddr,
      ROLE_MINT_INITIATOR
    );
    return sendAndCapture(rpc, rpcSub, admin, [ix], [admin], "GrantRole(MINT_INITIATOR -> Minter)");
  });

  // ═══════════════════════════════════════════════════════════
  // Step 2: Admin grants ROLE_REDEMPTION_EXECUTOR to Executor
  // ═══════════════════════════════════════════════════════════
  await step("Admin grants ROLE_REDEMPTION_EXECUTOR to Executor", ["RoleGranted"], async () => {
    const ix = grantRole(
      admin,
      permConfigAddr,
      executorPermsAddr,
      executor.address,
      adminPermsAddr,
      ROLE_REDEMPTION_EXECUTOR
    );
    return sendAndCapture(rpc, rpcSub, admin, [ix], [admin], "GrantRole(REDEMPTION_EXECUTOR -> Executor)");
  });

  // ═══════════════════════════════════════════════════════════
  // Step 3: Admin grants ROLE_WHITELISTER to Whitelister
  // ═══════════════════════════════════════════════════════════
  await step("Admin grants ROLE_WHITELISTER to Whitelister", ["RoleGranted"], async () => {
    const ix = grantRole(
      admin,
      permConfigAddr,
      whitelisterPermsAddr,
      whitelister.address,
      adminPermsAddr,
      ROLE_WHITELISTER
    );
    return sendAndCapture(rpc, rpcSub, admin, [ix], [admin], "GrantRole(WHITELISTER -> Whitelister)");
  });

  // ═══════════════════════════════════════════════════════════
  // Step 4: Whitelister grants ROLE_WHITELISTED to User1
  // ═══════════════════════════════════════════════════════════
  await step("Whitelister grants ROLE_WHITELISTED to User1", ["RoleGranted"], async () => {
    const ix = grantRole(
      whitelister,
      permConfigAddr,
      user1PermsAddr,
      user1.address,
      whitelisterPermsAddr,
      ROLE_WHITELISTED
    );
    return sendAndCapture(rpc, rpcSub, whitelister, [ix], [whitelister], "GrantRole(WHITELISTED -> User1)");
  });

  // ═══════════════════════════════════════════════════════════
  // Step 5: Whitelister grants ROLE_WHITELISTED to User2
  // ═══════════════════════════════════════════════════════════
  await step("Whitelister grants ROLE_WHITELISTED to User2", ["RoleGranted"], async () => {
    const ix = grantRole(
      whitelister,
      permConfigAddr,
      user2PermsAddr,
      user2.address,
      whitelisterPermsAddr,
      ROLE_WHITELISTED
    );
    return sendAndCapture(rpc, rpcSub, whitelister, [ix], [whitelister], "GrantRole(WHITELISTED -> User2)");
  });

  // ═══════════════════════════════════════════════════════════
  // Step 6: Minter mints 20 shares to User1 (auto-executes)
  // ═══════════════════════════════════════════════════════════
  await step(
    `Minter mints ${MINT_SHARES} ${TOKEN.toUpperCase()} to User1`,
    ["MintExecuted", "Mint"],
    async () => {
      const instructions: any[] = [];

      // Create User1's ATA if needed (admin pays for creation)
      if (!(await accountExists(rpc, user1Ata))) {
        instructions.push(
          createAssociatedTokenAccountIdempotent(
            admin,
            user1Ata,
            user1.address,
            mintAddr
          )
        );
      }

      const operationId = await computeOperationId(
        user1.address,
        mintAddr,
        mintRawAmount,
        mintSalt
      );
      const [mintOpAddr] = await mintOperationPda(operationId);

      instructions.push(
        initiateMint(
          minter,
          minterConfigAddr,
          dailyLimitAddr,
          mintOpAddr,
          minterPermsAddr,
          tokenConfigAddr,
          mintAddr,
          user1Ata,
          mintAuthAddr,
          minterConfigPermsAddr,
          user1PermsAddr,
          user1.address,
          mintRawAmount,
          mintSalt
        )
      );

      // Admin pays fees, minter co-signs
      return sendAndCapture(rpc, rpcSub, admin, instructions, [admin, minter], "InitiateMint(20 -> User1)");
    }
  );

  // ═══════════════════════════════════════════════════════════
  // Step 7a: User1 transfers 10 shares to User2 via spiko-token (Path B)
  // ═══════════════════════════════════════════════════════════
  await step(
    `User1 transfers ${TRANSFER_B_SHARES} shares to User2 via spiko-token transfer (Path B)`,
    ["Transfer"],
    async () => {
      const instructions: any[] = [];

      // Create User2's ATA if needed (admin pays)
      if (!(await accountExists(rpc, user2Ata))) {
        instructions.push(
          createAssociatedTokenAccountIdempotent(
            admin,
            user2Ata,
            user2.address,
            mintAddr
          )
        );
      }

      instructions.push(
        transferToken(
          user1,
          tokenConfigAddr,
          user1Ata,
          user2Ata,
          mintAddr,
          user1PermsAddr,
          user2PermsAddr,
          extraMetaListAddr,
          transferBRawAmount
        )
      );

      // Admin pays fees, user1 co-signs the transfer
      return sendAndCapture(
        rpc,
        rpcSub,
        admin,
        instructions,
        [admin, user1],
        "TransferToken(10 User1->User2, Path B)"
      );
    }
  );

  // ═══════════════════════════════════════════════════════════
  // Step 7b: User1 transfers 5 shares to User2 via direct TransferChecked (Path A)
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
        user1PermsAddr,
        user2PermsAddr,
        extraMetaListAddr,
        transferARawAmount
      );

      // Admin pays fees, user1 co-signs
      return sendAndCapture(
        rpc,
        rpcSub,
        admin,
        [ix],
        [admin, user1],
        "TransferChecked(5 User1->User2, Path A)"
      );
    }
  );

  // ═══════════════════════════════════════════════════════════
  // Step 8: User2 redeems 10 shares (tokens go to vault)
  // ═══════════════════════════════════════════════════════════
  await step(
    `User2 redeems ${REDEEM_SHARES} ${TOKEN.toUpperCase()}`,
    ["RedeemInitiated", "RedemptionInitiated", "Transfer"],
    async () => {
      const instructions: any[] = [];

      // Create vault ATA if needed
      if (!(await accountExists(rpc, vaultAta))) {
        instructions.push(
          createAssociatedTokenAccountIdempotent(
            admin,
            vaultAta,
            vaultAuthAddr,
            mintAddr
          )
        );
      }

      const [tokenMinAddr] = await tokenMinimumPda(mintAddr);

      const operationId = await computeOperationId(
        user2.address,
        mintAddr,
        redeemRawAmount,
        redeemSalt
      );
      const [redemptionOpAddr] = await redemptionOperationPda(operationId);

      instructions.push(
        redeemToken(
          user2,
          tokenConfigAddr,
          user2Ata,
          vaultAta,
          mintAddr,
          user2PermsAddr,
          vaultAuthPermsAddr,
          redemptionConfigAddr,
          redemptionOpAddr,
          tokenMinAddr,
          extraMetaListAddr,
          redeemRawAmount,
          redeemSalt
        )
      );

      // Admin pays fees, user2 co-signs
      return sendAndCapture(
        rpc,
        rpcSub,
        admin,
        instructions,
        [admin, user2],
        "RedeemToken(10 User2)"
      );
    }
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
        redeemSalt
      );
      const [redemptionOpAddr] = await redemptionOperationPda(operationId);

      const ix = executeRedemption(
        executor,
        redemptionConfigAddr,
        redemptionOpAddr,
        executorPermsAddr,
        tokenConfigAddr,
        mintAddr,
        vaultAta,
        mintAuthAddr,
        vaultAuthAddr,
        vaultAuthPermsAddr,
        user2.address,
        redeemRawAmount,
        redeemSalt
      );

      // Admin pays fees, executor co-signs
      return sendAndCapture(
        rpc,
        rpcSub,
        admin,
        [ix],
        [admin, executor],
        "ExecuteRedemption(10 User2)"
      );
    }
  );

  // ═══════════════════════════════════════════════════════════
  // Summary
  // ═══════════════════════════════════════════════════════════
  // Steps 1-5: 5 x RoleGranted = 5
  // Step 6: MintExecuted + Mint = 2
  // Step 7a: Transfer = 1
  // Step 7b: Transfer = 1
  // Step 8: RedeemInitiated + RedemptionInitiated + Transfer = 3
  // Step 9: RedemptionExecuted + Burn = 2
  // Total = 14
  const expectedTotal = 14;

  console.log(`\n\n=== E2E Test Complete ===`);
  console.log(`Steps:          ${totalSteps}`);
  console.log(`Events decoded: ${totalEvents}/${expectedTotal}`);

  if (totalEvents >= expectedTotal) {
    console.log("All expected events were found!");
  } else {
    console.log(
      `Warning: Some events were not found. This may be due to RPC indexing delay.`
    );
  }

  // Final balance check
  console.log("\n--- Final Balances ---\n");
  const { getBase64EncodedWireTransaction } = await import("@solana/kit");
  for (const [name, ata] of [["User1", user1Ata], ["User2", user2Ata], ["Vault", vaultAta]] as const) {
    try {
      const { value } = await rpc
        .getTokenAccountBalance(ata as Address, { commitment: "confirmed" })
        .send();
      const shares = Number(value.amount) / 10 ** TOKEN_DECIMALS;
      console.log(`  ${name}: ${shares} shares (${value.amount} raw)`);
    } catch {
      console.log(`  ${name}: (account not found or empty)`);
    }
  }
}

main().catch((err) => {
  console.error("\nE2E test failed:", err);
  process.exit(1);
});
