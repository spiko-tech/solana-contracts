/**
 * End-to-end smoke test on devnet.
 *
 * Runs a full lifecycle — whitelist, mint, burn, redeem, execute-redemption —
 * using the admin wallet, then fetches and decodes structured events from
 * each transaction's logs.
 *
 * Usage:
 *   npx tsx smoke-test.ts
 */

import {
  type Rpc,
  type SolanaRpcApi,
  type RpcSubscriptions,
  type SolanaRpcSubscriptionsApi,
  type KeyPairSigner,
  pipe,
  createTransactionMessage,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  appendTransactionMessageInstructions,
  signTransactionMessageWithSigners,
  sendAndConfirmTransactionFactory,
  getSignatureFromTransaction,
} from "@solana/kit";

import { TOKEN_DECIMALS, ROLE_WHITELISTED } from "./lib/constants.js";
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
  burnToken,
  redeemToken,
  executeRedemption,
} from "./lib/instructions.js";
import { initiateMint } from "./lib/instructions.js";
import {
  setup,
  resolveMintAddress,
  getAssociatedTokenAddress,
  createAssociatedTokenAccountIdempotent,
  accountExists,
  computeOperationId,
} from "./lib/shared.js";
import {
  parseTransactionEvents,
  formatEvents,
  type DecodedEvent,
} from "./lib/events.js";

// =================================================================
// Send + capture signature (same as shared.sendTx but returns sig)
// =================================================================

async function sendAndCapture(
  rpc: Rpc<SolanaRpcApi>,
  rpcSub: RpcSubscriptions<SolanaRpcSubscriptionsApi>,
  payer: KeyPairSigner,
  instructions: any[],
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

// =================================================================
// Main
// =================================================================

async function main() {
  console.log("=== Spiko Devnet Smoke Test ===\n");

  const { rpc, rpcSub, admin } = await setup();

  const TOKEN = "eutbl";
  const MINT_AMOUNT_SHARES = 100;
  const BURN_AMOUNT_SHARES = 10;
  const REDEEM_AMOUNT_SHARES = 10;

  const mintRawAmount = BigInt(
    Math.round(MINT_AMOUNT_SHARES * 10 ** TOKEN_DECIMALS)
  );
  const burnRawAmount = BigInt(
    Math.round(BURN_AMOUNT_SHARES * 10 ** TOKEN_DECIMALS)
  );
  const redeemRawAmount = BigInt(
    Math.round(REDEEM_AMOUNT_SHARES * 10 ** TOKEN_DECIMALS)
  );

  const mintSalt = BigInt(Math.floor(Math.random() * Number.MAX_SAFE_INTEGER));
  const redeemSalt = BigInt(
    Math.floor(Math.random() * Number.MAX_SAFE_INTEGER)
  );

  // Resolve addresses
  const mintAddr = await resolveMintAddress(TOKEN);
  const [permConfigAddr] = await permissionConfigPda();
  const [adminPermsAddr] = await userPermissionsPda(admin.address);
  const [minterConfigAddr] = await minterConfigPda();
  const [minterConfigPermsAddr] = await userPermissionsPda(minterConfigAddr);
  const [tokenConfigAddr] = await tokenConfigPda(mintAddr);
  const [mintAuthAddr] = await mintAuthorityPda(mintAddr);
  const [dailyLimitAddr] = await dailyLimitPda(mintAddr);
  const [redemptionConfigAddr] = await redemptionConfigPda();
  const [vaultAuthAddr] = await vaultAuthorityPda();
  const [vaultAuthPermsAddr] = await userPermissionsPda(vaultAuthAddr);
  const [extraMetaListAddr] = await extraAccountMetaListPda(mintAddr);

  const adminAta = await getAssociatedTokenAddress(admin.address, mintAddr);
  const vaultAta = await getAssociatedTokenAddress(vaultAuthAddr, mintAddr);

  console.log(`Token:          ${TOKEN.toUpperCase()}`);
  console.log(`Mint:           ${mintAddr}`);
  console.log(`Admin:          ${admin.address}`);
  console.log(`Admin ATA:      ${adminAta}`);
  console.log(`Vault ATA:      ${vaultAta}\n`);

  let totalEvents = 0;
  let totalSteps = 0;

  // Helper: run a step, fetch events, print them
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

    // Check expected events
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

  // ─── Step 1: Whitelist admin (idempotent) ──────────────────
  await step("Whitelist admin", ["RoleGranted"], async () => {
    const ix = grantRole(
      admin,
      permConfigAddr,
      adminPermsAddr,
      admin.address,
      adminPermsAddr,
      ROLE_WHITELISTED
    );
    return sendAndCapture(rpc, rpcSub, admin, [ix], "GrantRole(WHITELISTED)");
  });

  // ─── Step 2: Mint 100 EUTBL to admin ───────────────────────
  await step(
    `Mint ${MINT_AMOUNT_SHARES} ${TOKEN.toUpperCase()}`,
    ["MintExecuted", "Mint"],
    async () => {
      const instructions: any[] = [];

      // Create admin ATA if needed
      if (!(await accountExists(rpc, adminAta))) {
        instructions.push(
          createAssociatedTokenAccountIdempotent(
            admin,
            adminAta,
            admin.address,
            mintAddr
          )
        );
      }

      // Compute mint operation PDA
      const operationId = await computeOperationId(
        admin.address,
        mintAddr,
        mintRawAmount,
        mintSalt
      );
      const [mintOpAddr] = await mintOperationPda(operationId);

      instructions.push(
        initiateMint(
          admin,
          minterConfigAddr,
          dailyLimitAddr,
          mintOpAddr,
          adminPermsAddr,
          tokenConfigAddr,
          mintAddr,
          adminAta,
          mintAuthAddr,
          minterConfigPermsAddr,
          adminPermsAddr,
          admin.address,
          mintRawAmount,
          mintSalt
        )
      );

      return sendAndCapture(rpc, rpcSub, admin, instructions, "InitiateMint");
    }
  );

  // ─── Step 3: Burn 10 EUTBL from admin ──────────────────────
  await step(
    `Burn ${BURN_AMOUNT_SHARES} ${TOKEN.toUpperCase()}`,
    ["Burn"],
    async () => {
      const ix = burnToken(
        admin,
        tokenConfigAddr,
        mintAddr,
        adminAta,
        mintAuthAddr,
        adminPermsAddr,
        burnRawAmount
      );

      return sendAndCapture(rpc, rpcSub, admin, [ix], "BurnToken");
    }
  );

  // ─── Step 4: Redeem 10 EUTBL ──────────────────────────────
  await step(
    `Redeem ${REDEEM_AMOUNT_SHARES} ${TOKEN.toUpperCase()}`,
    ["RedeemInitiated", "RedemptionInitiated"],
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

      // Compute redemption operation PDA
      const operationId = await computeOperationId(
        admin.address,
        mintAddr,
        redeemRawAmount,
        redeemSalt
      );
      const [redemptionOpAddr] = await redemptionOperationPda(operationId);

      instructions.push(
        redeemToken(
          admin,
          tokenConfigAddr,
          adminAta,
          vaultAta,
          mintAddr,
          adminPermsAddr,
          vaultAuthPermsAddr,
          redemptionConfigAddr,
          redemptionOpAddr,
          tokenMinAddr,
          extraMetaListAddr,
          redeemRawAmount,
          redeemSalt
        )
      );

      return sendAndCapture(rpc, rpcSub, admin, instructions, "RedeemToken");
    }
  );

  // ─── Step 5: Execute redemption ────────────────────────────
  await step(
    "Execute redemption",
    ["RedemptionExecuted", "Burn"],
    async () => {
      const operationId = await computeOperationId(
        admin.address,
        mintAddr,
        redeemRawAmount,
        redeemSalt
      );
      const [redemptionOpAddr] = await redemptionOperationPda(operationId);

      const ix = executeRedemption(
        admin,
        redemptionConfigAddr,
        redemptionOpAddr,
        adminPermsAddr,
        tokenConfigAddr,
        mintAddr,
        vaultAta,
        mintAuthAddr,
        vaultAuthAddr,
        vaultAuthPermsAddr,
        admin.address,
        redeemRawAmount,
        redeemSalt
      );

      return sendAndCapture(
        rpc,
        rpcSub,
        admin,
        [ix],
        "ExecuteRedemption"
      );
    }
  );

  // ─── Summary ───────────────────────────────────────────────
  const expectedTotal = 7; // RoleGranted + MintExecuted + Mint + Burn + RedeemInitiated + RedemptionInitiated + RedemptionExecuted + Burn(from execute)
  // Actually: Step1=1, Step2=2, Step3=1, Step4=2, Step5=2 = 8
  const expectedTotal2 = 8;
  console.log(`\n\n=== Smoke Test Complete ===`);
  console.log(`Events decoded: ${totalEvents}/${expectedTotal2}`);

  if (totalEvents >= expectedTotal2) {
    console.log("All expected events were found!");
  } else {
    console.log(
      `Warning: Some events were not found. This may be due to RPC indexing delay.`
    );
    console.log(
      `Try re-fetching the transactions after a few seconds.`
    );
  }
}

main().catch((err) => {
  console.error("\nSmoke test failed:", err);
  process.exit(1);
});
