/**
 * End-to-end test for all 4 Spiko Solana programs.
 *
 * Runs against whatever network the Solana CLI is configured for
 * (devnet or localnet).
 *
 * Usage:
 *   cd e2e && pnpm tsx e2e.ts
 */

import {
  type Address,
  type KeyPairSigner,
  type Rpc,
  type SolanaRpcApi,
  type RpcSubscriptions,
  type SolanaRpcSubscriptionsApi,
  type ReadonlyUint8Array,
  AccountRole,
} from "@solana/kit";

import {
  // Setup
  setup,
  generateKeyPairSigner,
  sendTx,
  sendTxExpectFail,
  getTokenBalance,
  getMintSupply,
  getMinimumRent,
  getAssociatedTokenAddress,
  accountExists,

  // Constants
  TOKEN_2022_PROGRAM_ID,
  SYSTEM_PROGRAM_ID,
  TRANSFER_HOOK_PROGRAM_ADDRESS,
  MINTER_PROGRAM_ADDRESS,
  REDEMPTION_PROGRAM_ADDRESS,
  CUSTODIAL_GATEKEEPER_PROGRAM_ADDRESS,

  // Token-2022 helpers
  buildCreateAccountInstruction,
  buildInitializeTransferHook,
  buildInitializePermanentDelegate,
  buildInitializeMetadataPointer,
  buildInitializeMint2,
  buildTokenMetadataInitialize,
  buildTransferChecked,
  buildSetAuthority,
  appendHookAccounts,
  createAssociatedTokenAccountIdempotent,
  MINT_FIXED_EXTENSIONS_SIZE,
  getMintAccountSpace,

  // PDA helpers
  findHookConfigPda,
  findExtraAccountMetasPda,
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

  // Instruction builders
  getThInitializeInstructionAsync,
  getRegisterMintInstructionAsync,
  getWhitelistInstructionAsync,
  getAddGateInstructionAsync,
  getMtInitializeInstructionAsync,
  getInitiateMintInstructionAsync,
  getApproveMintInstructionAsync,
  getCancelMintInstructionAsync,
  getMtSetDailyLimitInstructionAsync,
  getRdInitializeInstructionAsync,
  getRedeemInstructionAsync,
  getCgInitializeInstructionAsync,
  getInitiateGatekeepInstructionAsync,
  getApproveGatekeepInstructionAsync,
  getCancelGatekeepInstructionAsync,
  getCgSetDailyLimitInstructionAsync,
} from "./lib/shared.js";

// ═══════════════════════════════════════════════════════════════
// Config
// ═══════════════════════════════════════════════════════════════

const DECIMALS = 6;
const DAILY_LIMIT_MINTER = 1_000_000n * 10n ** BigInt(DECIMALS); // 1M tokens
const DAILY_LIMIT_CG = 500_000n * 10n ** BigInt(DECIMALS); // 500K tokens
const SMALL_AMOUNT = 100_000n * 10n ** BigInt(DECIMALS); // 100K tokens (under limits)
const LARGE_AMOUNT = 2_000_000n * 10n ** BigInt(DECIMALS); // 2M tokens (over minter limit)
const LARGE_AMOUNT_CG = 600_000n * 10n ** BigInt(DECIMALS); // 600K tokens (over CG limit)
const TRANSFER_AMOUNT = 10_000n * 10n ** BigInt(DECIMALS); // 10K tokens
const REDEEM_AMOUNT = 5_000n * 10n ** BigInt(DECIMALS); // 5K tokens

let saltCounter = Date.now();
function nextSalt(): bigint {
  return BigInt(saltCounter++);
}

// ═══════════════════════════════════════════════════════════════
// Main
// ═══════════════════════════════════════════════════════════════

async function main() {
  const { rpc, rpcSub, admin } = await setup();

  // Generate wallets
  const w1 = await generateKeyPairSigner();
  const w2 = await generateKeyPairSigner();
  const w3 = await generateKeyPairSigner();
  console.log(`w1 (custodial):      ${w1.address}`);
  console.log(`w2 (custodial):      ${w2.address}`);
  console.log(`w3 (self-custodial): ${w3.address}`);
  console.log();

  // Airdrop SOL to wallets (for signing transfers)
  await airdrop(rpc, rpcSub, admin, w1.address, 5_000_000n); // 0.005 SOL
  await airdrop(rpc, rpcSub, admin, w2.address, 5_000_000n);
  await airdrop(rpc, rpcSub, admin, w3.address, 5_000_000n);

  // Run full flow for EUTBL
  await runTokenFlow(rpc, rpcSub, admin, w1, w2, w3, "EUTBL", "EU T-Bill Token", "https://spiko.finance/eutbl");

  // Run full flow for USTBL
  await runTokenFlow(rpc, rpcSub, admin, w1, w2, w3, "USTBL", "US T-Bill Token", "https://spiko.finance/ustbl");

  console.log("\n═══════════════════════════════════════════════════════");
  console.log("ALL E2E TESTS PASSED");
  console.log("═══════════════════════════════════════════════════════\n");
}

// ═══════════════════════════════════════════════════════════════
// Token flow (runs for each token: EUTBL, USTBL)
// ═══════════════════════════════════════════════════════════════

async function runTokenFlow(
  rpc: Rpc<SolanaRpcApi>,
  rpcSub: RpcSubscriptions<SolanaRpcSubscriptionsApi>,
  admin: KeyPairSigner,
  w1: KeyPairSigner,
  w2: KeyPairSigner,
  w3: KeyPairSigner,
  symbol: string,
  name: string,
  uri: string,
) {
  console.log(`\n${"═".repeat(60)}`);
  console.log(`TOKEN: ${symbol}`);
  console.log(`${"═".repeat(60)}`);

  // ─── Phase 1: Setup ──────────────────────────────────────────
  console.log("\n── Phase 1: Setup ──");

  // 1. Create mint (admin as initial authority, then transfer to minter config PDA)
  const mintKp = await generateKeyPairSigner();
  const mint = mintKp.address;
  console.log(`  Mint: ${mint}`);

  const [minterConfigAddr] = await findMinterConfigPda();

  // Create mint account with extensions (admin as mint authority initially)
  // Fund for full size including metadata (Token-2022 will realloc)
  const fullMintSize = getMintAccountSpace(name, symbol, uri);
  const mintRent = await getMinimumRent(rpc, fullMintSize);
  await sendTx(rpc, rpcSub, admin, [
    buildCreateAccountInstruction(admin, mintKp, mintRent, MINT_FIXED_EXTENSIONS_SIZE, TOKEN_2022_PROGRAM_ID),
    buildInitializeTransferHook(mint, admin.address, TRANSFER_HOOK_PROGRAM_ADDRESS as Address),
    buildInitializePermanentDelegate(mint, admin.address),
    buildInitializeMetadataPointer(mint, admin.address, mint),
    buildInitializeMint2(mint, DECIMALS, admin.address),
  ], `Create mint ${symbol}`);

  // Add metadata (requires mint authority = admin to sign)
  await sendTx(rpc, rpcSub, admin, [
    buildTokenMetadataInitialize(mint, admin.address, admin, name, symbol, uri),
  ], `Init metadata ${symbol}`);

  // 2. Initialize transfer-hook (skip if already initialized)
  const [hookConfigAddr] = await findHookConfigPda();
  if (!(await accountExists(rpc, hookConfigAddr))) {
    const thInitIx = await getThInitializeInstructionAsync({
      admin,
      whitelistAuthority: admin.address,
    });
    await sendTx(rpc, rpcSub, admin, [thInitIx], "Init transfer-hook");
  } else {
    console.log("  Transfer-hook already initialized, skipping");
  }

  // Register mint with transfer-hook
  const regMintIx = await getRegisterMintInstructionAsync({
    admin,
    mint,
  });
  await sendTx(rpc, rpcSub, admin, [regMintIx], `Register mint ${symbol}`);

  // 3. Initialize minter (must happen before transferring mint authority)
  const [minterConfigAddrCheck] = await findMinterConfigPda();
  if (!(await accountExists(rpc, minterConfigAddrCheck))) {
    const mtInitIx = await getMtInitializeInstructionAsync({
      admin,
      mintInitiator: admin.address,
    });
    await sendTx(rpc, rpcSub, admin, [mtInitIx], "Init minter");
  } else {
    console.log("  Minter already initialized, skipping");
  }

  // Transfer mint authority from admin to minter config PDA
  await sendTx(rpc, rpcSub, admin, [
    buildSetAuthority(mint, admin, 0 /* MintTokens */, minterConfigAddr),
  ], "Transfer mint authority to minter config");

  // Set daily limit for minter
  const mtLimitIx = await getMtSetDailyLimitInstructionAsync({
    admin,
    mint,
    payer: admin,
    limit: DAILY_LIMIT_MINTER,
  });
  await sendTx(rpc, rpcSub, admin, [mtLimitIx], `Set minter daily limit ${symbol}`);

  // 4. Initialize redemption (skip if already initialized)
  const [rdConfigAddr] = await findRedemptionConfigPda();
  if (!(await accountExists(rpc, rdConfigAddr))) {
    const rdInitIx = await getRdInitializeInstructionAsync({
      admin,
      redemptionAuthority: admin.address,
    });
    await sendTx(rpc, rpcSub, admin, [rdInitIx], "Init redemption");
  } else {
    console.log("  Redemption already initialized, skipping");
  }

  // 5. Initialize custodial-gatekeeper (skip if already initialized)
  const [cgConfigAddr] = await findGatekeeperConfigPda();
  if (!(await accountExists(rpc, cgConfigAddr))) {
    const cgInitIx = await getCgInitializeInstructionAsync({
      admin,
      gatekeeperInitiator: admin.address,
    });
    await sendTx(rpc, rpcSub, admin, [cgInitIx], "Init custodial-gatekeeper");
  } else {
    console.log("  Custodial-gatekeeper already initialized, skipping");
  }

  // Set daily limit for custodial-gatekeeper
  const cgLimitIx = await getCgSetDailyLimitInstructionAsync({
    admin,
    mint,
    payer: admin,
    limit: DAILY_LIMIT_CG,
  });
  await sendTx(rpc, rpcSub, admin, [cgLimitIx], `Set CG daily limit ${symbol}`);

  // 6. Whitelist wallets
  // w1, w2: WHITELISTED (ext=false)
  const wl1Ix = await getWhitelistInstructionAsync({ authority: admin, wallet: w1.address, payer: admin, ext: false });
  const wl2Ix = await getWhitelistInstructionAsync({ authority: admin, wallet: w2.address, payer: admin, ext: false });
  // w3: WHITELISTED_EXT (ext=true)
  const wl3Ix = await getWhitelistInstructionAsync({ authority: admin, wallet: w3.address, payer: admin, ext: true });
  await sendTx(rpc, rpcSub, admin, [wl1Ix, wl2Ix, wl3Ix], "Whitelist w1, w2, w3");

  // 7. Whitelist redemption vault (WHITELISTED_GATE) - skip if already done
  const [rdVaultAuthorityAddr] = await findRedemptionVaultAuthorityPda();
  const [rdVaultWlPda] = await findWhitelistStatePda({ wallet: rdVaultAuthorityAddr });
  if (!(await accountExists(rpc, rdVaultWlPda))) {
    const gateRdVaultIx = await getAddGateInstructionAsync({ admin, wallet: rdVaultAuthorityAddr, payer: admin });
    await sendTx(rpc, rpcSub, admin, [gateRdVaultIx], "Gate redemption vault (WHITELISTED_GATE)");
  } else {
    console.log("  Redemption vault already whitelisted, skipping");
  }

  // 8. Whitelist custodial-gatekeeper vault (WHITELISTED_GATE) - skip if already done
  const [cgVaultAuthorityAddr] = await findCgVaultAuthorityPda();
  const [cgVaultWlPda] = await findWhitelistStatePda({ wallet: cgVaultAuthorityAddr });
  if (!(await accountExists(rpc, cgVaultWlPda))) {
    const gateCgVaultIx = await getAddGateInstructionAsync({ admin, wallet: cgVaultAuthorityAddr, payer: admin });
    await sendTx(rpc, rpcSub, admin, [gateCgVaultIx], "Gate CG vault (WHITELISTED_GATE)");
  } else {
    console.log("  CG vault already whitelisted, skipping");
  }

  // 9. Create ATAs for w1, w2, w3
  const w1Ata = await getAssociatedTokenAddress(w1.address, mint);
  const w2Ata = await getAssociatedTokenAddress(w2.address, mint);
  const w3Ata = await getAssociatedTokenAddress(w3.address, mint);
  await sendTx(rpc, rpcSub, admin, [
    createAssociatedTokenAccountIdempotent(admin, w1Ata, w1.address, mint),
    createAssociatedTokenAccountIdempotent(admin, w2Ata, w2.address, mint),
    createAssociatedTokenAccountIdempotent(admin, w3Ata, w3.address, mint),
  ], "Create ATAs for w1, w2, w3");

  // 10. Create vault token accounts (ATAs owned by vault authority PDAs)
  const rdVault = await getAssociatedTokenAddress(rdVaultAuthorityAddr, mint);
  const cgVault = await getAssociatedTokenAddress(cgVaultAuthorityAddr, mint);
  await sendTx(rpc, rpcSub, admin, [
    createAssociatedTokenAccountIdempotent(admin, rdVault, rdVaultAuthorityAddr, mint),
    createAssociatedTokenAccountIdempotent(admin, cgVault, cgVaultAuthorityAddr, mint),
  ], "Create vault ATAs (redemption + CG)");

  // Get PDA addresses for transfers
  const [extraMetasAddr] = await findExtraAccountMetasPda({ mint });
  const [w1WhitelistAddr] = await findWhitelistStatePda({ wallet: w1.address });
  const [w2WhitelistAddr] = await findWhitelistStatePda({ wallet: w2.address });
  const [w3WhitelistAddr] = await findWhitelistStatePda({ wallet: w3.address });
  const [rdVaultWhitelistAddr] = await findWhitelistStatePda({ wallet: rdVaultAuthorityAddr });
  const [cgVaultWhitelistAddr] = await findWhitelistStatePda({ wallet: cgVaultAuthorityAddr });

  // ─── Phase 2: Minting ────────────────────────────────────────
  console.log("\n── Phase 2: Minting ──");

  // Mint to w1 (under limit → auto-approved)
  const salt1 = nextSalt();
  const mintW1Ix = await getInitiateMintInstructionAsync({
    initiator: admin,
    mint,
    destination: w1Ata,
    payer: admin,
    salt: salt1,
    recipient: w1.address,
    amount: SMALL_AMOUNT,
    tokenProgram: TOKEN_2022_PROGRAM_ID,
  });
  await sendTx(rpc, rpcSub, admin, [mintW1Ix], "Mint to w1 (auto-approved)");
  assert(await getTokenBalance(rpc, w1Ata) === SMALL_AMOUNT, "w1 balance after mint");

  // Mint to w2 (over limit → PENDING)
  const salt2 = nextSalt();
  const mintW2Ix = await getInitiateMintInstructionAsync({
    initiator: admin,
    mint,
    destination: w2Ata,
    payer: admin,
    salt: salt2,
    recipient: w2.address,
    amount: LARGE_AMOUNT,
    tokenProgram: TOKEN_2022_PROGRAM_ID,
  });
  await sendTx(rpc, rpcSub, admin, [mintW2Ix], "Mint to w2 (PENDING - over limit)");
  assert(await getTokenBalance(rpc, w2Ata) === 0n, "w2 balance still 0 (pending)");

  // Approve mint for w2
  const approveW2Ix = await getApproveMintInstructionAsync({
    admin,
    mint,
    destination: w2Ata,
    salt: salt2,
    tokenProgram: TOKEN_2022_PROGRAM_ID,
  });
  await sendTx(rpc, rpcSub, admin, [approveW2Ix], "Approve mint for w2");
  assert(await getTokenBalance(rpc, w2Ata) === LARGE_AMOUNT, "w2 balance after approve");

  // Mint to w3 (under limit → auto-approved)
  const salt3 = nextSalt();
  const mintW3Ix = await getInitiateMintInstructionAsync({
    initiator: admin,
    mint,
    destination: w3Ata,
    payer: admin,
    salt: salt3,
    recipient: w3.address,
    amount: SMALL_AMOUNT,
    tokenProgram: TOKEN_2022_PROGRAM_ID,
  });
  await sendTx(rpc, rpcSub, admin, [mintW3Ix], "Mint to w3 (auto-approved)");
  assert(await getTokenBalance(rpc, w3Ata) === SMALL_AMOUNT, "w3 balance after mint");

  // Mint to w2 over limit → PENDING → cancel
  const salt4 = nextSalt();
  const mintW2bIx = await getInitiateMintInstructionAsync({
    initiator: admin,
    mint,
    destination: w2Ata,
    payer: admin,
    salt: salt4,
    recipient: w2.address,
    amount: LARGE_AMOUNT,
    tokenProgram: TOKEN_2022_PROGRAM_ID,
  });
  await sendTx(rpc, rpcSub, admin, [mintW2bIx], "Mint to w2 (PENDING - will cancel)");
  const w2BalBeforeCancel = await getTokenBalance(rpc, w2Ata);

  const cancelMintIx = await getCancelMintInstructionAsync({
    admin,
    salt: salt4,
  });
  await sendTx(rpc, rpcSub, admin, [cancelMintIx], "Cancel mint for w2");
  assert(await getTokenBalance(rpc, w2Ata) === w2BalBeforeCancel, "w2 balance unchanged after cancel");

  console.log("  Minting phase PASSED");

  // ─── Phase 3: Transfers ──────────────────────────────────────
  console.log("\n── Phase 3: Transfers ──");

  // w1 → w2 direct (whitelisted → whitelisted ✓)
  await sendTx(rpc, rpcSub, w1, [
    buildTransferChecked(w1, w1Ata, w2Ata, mint, extraMetasAddr, hookConfigAddr, w1WhitelistAddr, w2WhitelistAddr, TRANSFER_AMOUNT, DECIMALS),
  ], "w1 → w2 direct transfer (OK)");

  // w2 → w3 direct should FAIL (whitelisted → whitelisted_ext ✗)
  const failResult = await sendTxExpectFail(rpc, rpcSub, w2, [
    buildTransferChecked(w2, w2Ata, w3Ata, mint, extraMetasAddr, hookConfigAddr, w2WhitelistAddr, w3WhitelistAddr, TRANSFER_AMOUNT, DECIMALS),
  ], "w2 → w3 direct transfer (SHOULD FAIL)");
  assert(failResult, "w2→w3 direct transfer should have failed");

  // w2 → CG vault (whitelisted → whitelisted_gate ✓)
  const cgVaultAta = cgVault;
  await sendTx(rpc, rpcSub, w2, [
    buildTransferChecked(w2, w2Ata, cgVaultAta, mint, extraMetasAddr, hookConfigAddr, w2WhitelistAddr, cgVaultWhitelistAddr, TRANSFER_AMOUNT, DECIMALS),
  ], "w2 → CG vault (OK)");

  // Gatekeeper initiates gatekeep (under CG daily limit → auto-transfer to w3)
  const saltCg1 = nextSalt();
  let cgInitiateIx1 = await getInitiateGatekeepInstructionAsync({
    gatekeeperInitiator: admin,
    mint,
    vault: cgVaultAta,
    destinationTokenAccount: w3Ata,
    payer: admin,
    salt: saltCg1,
    sender: w2.address,
    destination: w3.address,
    amount: TRANSFER_AMOUNT,
    tokenProgram: TOKEN_2022_PROGRAM_ID,
  });
  cgInitiateIx1 = appendHookAccounts(cgInitiateIx1, extraMetasAddr, hookConfigAddr, cgVaultWhitelistAddr, w3WhitelistAddr);
  await sendTx(rpc, rpcSub, admin, [cgInitiateIx1], "Initiate gatekeep under limit (auto → w3)");

  // w2 → CG vault again (larger amount, over CG daily limit)
  await sendTx(rpc, rpcSub, w2, [
    buildTransferChecked(w2, w2Ata, cgVaultAta, mint, extraMetasAddr, hookConfigAddr, w2WhitelistAddr, cgVaultWhitelistAddr, LARGE_AMOUNT_CG, DECIMALS),
  ], "w2 → CG vault (large amount)");

  // Gatekeeper initiates gatekeep (over limit → PENDING)
  const saltCg2 = nextSalt();
  let cgInitiateIx2 = await getInitiateGatekeepInstructionAsync({
    gatekeeperInitiator: admin,
    mint,
    vault: cgVaultAta,
    destinationTokenAccount: w3Ata,
    payer: admin,
    salt: saltCg2,
    sender: w2.address,
    destination: w3.address,
    amount: LARGE_AMOUNT_CG,
    tokenProgram: TOKEN_2022_PROGRAM_ID,
  });
  cgInitiateIx2 = appendHookAccounts(cgInitiateIx2, extraMetasAddr, hookConfigAddr, cgVaultWhitelistAddr, w3WhitelistAddr);
  await sendTx(rpc, rpcSub, admin, [cgInitiateIx2], "Initiate gatekeep over limit (PENDING)");

  // Approve gatekeep → transfers from vault to w3
  let cgApproveIx = await getApproveGatekeepInstructionAsync({
    admin,
    mint,
    vault: cgVaultAta,
    destinationTokenAccount: w3Ata,
    salt: saltCg2,
    tokenProgram: TOKEN_2022_PROGRAM_ID,
  });
  cgApproveIx = appendHookAccounts(cgApproveIx, extraMetasAddr, hookConfigAddr, cgVaultWhitelistAddr, w3WhitelistAddr);
  await sendTx(rpc, rpcSub, admin, [cgApproveIx], "Approve gatekeep (vault → w3)");

  // Test cancel: w2 → CG vault, initiate over limit, cancel → refund to w2
  await sendTx(rpc, rpcSub, w2, [
    buildTransferChecked(w2, w2Ata, cgVaultAta, mint, extraMetasAddr, hookConfigAddr, w2WhitelistAddr, cgVaultWhitelistAddr, LARGE_AMOUNT_CG, DECIMALS),
  ], "w2 → CG vault (for cancel test)");

  const w2BalBeforeCgCancel = await getTokenBalance(rpc, w2Ata);

  const saltCg3 = nextSalt();
  let cgInitiateIx3 = await getInitiateGatekeepInstructionAsync({
    gatekeeperInitiator: admin,
    mint,
    vault: cgVaultAta,
    destinationTokenAccount: w3Ata,
    payer: admin,
    salt: saltCg3,
    sender: w2.address,
    destination: w3.address,
    amount: LARGE_AMOUNT_CG,
    tokenProgram: TOKEN_2022_PROGRAM_ID,
  });
  cgInitiateIx3 = appendHookAccounts(cgInitiateIx3, extraMetasAddr, hookConfigAddr, cgVaultWhitelistAddr, w3WhitelistAddr);
  await sendTx(rpc, rpcSub, admin, [cgInitiateIx3], "Initiate gatekeep (PENDING - will cancel)");

  let cgCancelIx = await getCancelGatekeepInstructionAsync({
    admin,
    mint,
    vault: cgVaultAta,
    senderTokenAccount: w2Ata,
    salt: saltCg3,
    tokenProgram: TOKEN_2022_PROGRAM_ID,
  });
  cgCancelIx = appendHookAccounts(cgCancelIx, extraMetasAddr, hookConfigAddr, cgVaultWhitelistAddr, w2WhitelistAddr);
  await sendTx(rpc, rpcSub, admin, [cgCancelIx], "Cancel gatekeep (refund → w2)");
  assert(await getTokenBalance(rpc, w2Ata) === w2BalBeforeCgCancel + LARGE_AMOUNT_CG, "w2 refunded after cancel");

  // w3 → w1 direct (whitelisted_ext → whitelisted ✓)
  await sendTx(rpc, rpcSub, w3, [
    buildTransferChecked(w3, w3Ata, w1Ata, mint, extraMetasAddr, hookConfigAddr, w3WhitelistAddr, w1WhitelistAddr, TRANSFER_AMOUNT, DECIMALS),
  ], "w3 → w1 direct transfer (OK)");

  console.log("  Transfers phase PASSED");

  // ─── Phase 4: Redemption ─────────────────────────────────────
  console.log("\n── Phase 4: Redemption ──");

  // w1 → redemption vault (whitelisted → whitelisted_gate ✓)
  const rdVaultAta = rdVault;
  await sendTx(rpc, rpcSub, w1, [
    buildTransferChecked(w1, w1Ata, rdVaultAta, mint, extraMetasAddr, hookConfigAddr, w1WhitelistAddr, rdVaultWhitelistAddr, REDEEM_AMOUNT, DECIMALS),
  ], "w1 → redemption vault");

  const supplyBefore = await getMintSupply(rpc, mint);
  const vaultBalBefore = await getTokenBalance(rpc, rdVaultAta);
  assert(vaultBalBefore >= REDEEM_AMOUNT, "vault has tokens");

  // Redemption authority calls redeem (burns from vault)
  const saltRd = nextSalt();
  const redeemIx = await getRedeemInstructionAsync({
    authority: admin,
    mint,
    vault: rdVaultAta,
    payer: admin,
    salt: saltRd,
    amount: REDEEM_AMOUNT,
    tokenProgram: TOKEN_2022_PROGRAM_ID,
  });
  await sendTx(rpc, rpcSub, admin, [redeemIx], "Redeem (burn from vault)");

  const supplyAfter = await getMintSupply(rpc, mint);
  const vaultBalAfter = await getTokenBalance(rpc, rdVaultAta);
  assert(supplyAfter === supplyBefore - REDEEM_AMOUNT, "supply decreased after redeem");
  assert(vaultBalAfter === vaultBalBefore - REDEEM_AMOUNT, "vault balance decreased after redeem");

  console.log("  Redemption phase PASSED");

  console.log(`\n  ${symbol} flow COMPLETE`);
}

// ═══════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════

function assert(condition: boolean, msg: string) {
  if (!condition) {
    throw new Error(`ASSERTION FAILED: ${msg}`);
  }
  console.log(`  ✓ ${msg}`);
}

/**
 * Transfer SOL from admin to a wallet (simple system transfer).
 */
async function airdrop(
  rpc: Rpc<SolanaRpcApi>,
  rpcSub: RpcSubscriptions<SolanaRpcSubscriptionsApi>,
  admin: KeyPairSigner,
  to: Address,
  lamports: bigint,
) {
  const data = new Uint8Array(4 + 8);
  const view = new DataView(data.buffer);
  view.setUint32(0, 2, true); // SystemProgram::Transfer
  view.setBigUint64(4, lamports, true);

  const ix = {
    programAddress: SYSTEM_PROGRAM_ID,
    accounts: [
      { address: admin.address, role: AccountRole.WRITABLE_SIGNER as const, signer: admin },
      { address: to, role: AccountRole.WRITABLE as const },
    ],
    data: data as ReadonlyUint8Array,
  };

  await sendTx(rpc, rpcSub, admin, [ix], `Airdrop ${lamports} lamports to ${to.slice(0, 8)}...`);
}

// ═══════════════════════════════════════════════════════════════

main().catch((err) => {
  console.error("\nE2E TEST FAILED:", err);
  process.exit(1);
});
