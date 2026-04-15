/**
 * Initiate a token redemption via the SpikoToken program.
 *
 * This transfers tokens from the user's ATA to the vault, then CPIs into
 * Redemption::on_redeem to create a PENDING RedemptionOperation with a
 * 14-day deadline.
 *
 * Prerequisites:
 *   - deploy.ts has been run (all PDAs initialized, including Phase 8 vault authority roles)
 *   - User is whitelisted (ROLE_WHITELISTED)
 *   - Vault authority PDA is whitelisted (granted in deploy.ts Phase 8)
 *   - Vault ATA exists for this mint (created idempotently by this script)
 *
 * Usage:
 *   npx tsx scripts/redeem.ts <eutbl|ustbl> <AMOUNT_IN_SHARES> [SALT]
 *
 * Example:
 *   npx tsx scripts/redeem.ts eutbl 10
 *   # Redeems 10 shares (= 1,000,000 raw) — creates a PENDING redemption operation
 */

import {
  userPermissionsPda,
  tokenConfigPda,
  extraAccountMetaListPda,
  redemptionConfigPda,
  tokenMinimumPda,
  vaultAuthorityPda,
  redemptionOperationPda,
  spikoTokenEventAuthorityPda,
  redemptionEventAuthorityPda,
  transferHookEventAuthorityPda,
} from "./lib/pda.js";
import { redeemToken } from "./lib/instructions.js";
import {
  setup,
  sendTx,
  resolveMintAddress,
  getAssociatedTokenAddress,
  createAssociatedTokenAccountIdempotent,
  accountExists,
  computeOperationId,
  readMintDecimals,
} from "./lib/shared.js";

async function main() {
  const tokenName = process.argv[2];
  const amountArg = process.argv[3];
  const saltArg = process.argv[4];

  if (!tokenName || !amountArg) {
    console.error(
      "Usage: npx tsx scripts/redeem.ts <eutbl|ustbl> <AMOUNT_IN_SHARES> [SALT]"
    );
    console.error("Example: npx tsx scripts/redeem.ts eutbl 10");
    process.exit(1);
  }

  const amountShares = parseFloat(amountArg);
  const salt = saltArg
    ? BigInt(saltArg)
    : BigInt(Math.floor(Math.random() * Number.MAX_SAFE_INTEGER));

  const { rpc, rpcSub, admin } = await setup();

  // Admin is the user for devnet testing
  const user = admin;

  // Resolve mint address and read decimals from on-chain mint account
  const mintAddr = await resolveMintAddress(tokenName);
  const decimals = await readMintDecimals(rpc, mintAddr);
  const rawAmount = BigInt(Math.round(amountShares * 10 ** decimals));

  console.log(`=== Redeem Tokens ===\n`);
  console.log(`Token:   ${tokenName.toUpperCase()}`);
  console.log(`Amount:  ${amountShares} shares (raw: ${rawAmount})`);
  console.log(`Salt:    ${salt}\n`);

  console.log(`Mint:              ${mintAddr}`);

  // Derive all PDAs
  const [tokenConfigAddr] = await tokenConfigPda(mintAddr);
  const [userPermsAddr] = await userPermissionsPda(user.address);
  const [vaultAuthAddr] = await vaultAuthorityPda();
  const [vaultAuthPermsAddr] = await userPermissionsPda(vaultAuthAddr);
  const [redemptionConfigAddr] = await redemptionConfigPda();
  const [tokenMinAddr] = await tokenMinimumPda(mintAddr);
  const [extraMetaListAddr] = await extraAccountMetaListPda(mintAddr);
  const [stEventAuth] = await spikoTokenEventAuthorityPda();
  const [rdEventAuth] = await redemptionEventAuthorityPda();
  const [thEventAuth] = await transferHookEventAuthorityPda();

  // Compute operation_id for RedemptionOperation PDA
  const operationId = await computeOperationId(
    user.address,
    mintAddr,
    rawAmount,
    salt
  );
  const [redemptionOpAddr] = await redemptionOperationPda(operationId);

  // Derive ATAs
  const userSourceAta = await getAssociatedTokenAddress(user.address, mintAddr);
  const vaultAta = await getAssociatedTokenAddress(vaultAuthAddr, mintAddr);

  console.log(`TokenConfig:       ${tokenConfigAddr}`);
  console.log(`UserPerms:         ${userPermsAddr}`);
  console.log(`VaultAuthority:    ${vaultAuthAddr}`);
  console.log(`VaultAuthPerms:    ${vaultAuthPermsAddr}`);
  console.log(`RedemptionConfig:  ${redemptionConfigAddr}`);
  console.log(`TokenMinimum:      ${tokenMinAddr}`);
  console.log(`ExtraMetaList:     ${extraMetaListAddr}`);
  console.log(`RedemptionOp:      ${redemptionOpAddr}`);
  console.log(`UserSourceATA:     ${userSourceAta}`);
  console.log(`VaultATA:          ${vaultAta}\n`);

  // Build instructions
  const instructions: any[] = [];

  // Step 1: Create vault ATA if it doesn't exist
  if (!(await accountExists(rpc, vaultAta))) {
    console.log("Creating vault Associated Token Account...");
    const createAtaIx = createAssociatedTokenAccountIdempotent(
      admin,
      vaultAta,
      vaultAuthAddr,
      mintAddr
    );
    instructions.push(createAtaIx);
  } else {
    console.log("Vault ATA already exists.");
  }

  // Step 2: Build Redeem instruction
  console.log("Building RedeemToken instruction...");
  const ix = redeemToken(
    user,
    tokenConfigAddr,
    userSourceAta,
    vaultAta,
    mintAddr,
    userPermsAddr,
    vaultAuthPermsAddr,
    redemptionConfigAddr,
    redemptionOpAddr,
    tokenMinAddr,
    extraMetaListAddr,
    rawAmount,
    salt,
    thEventAuth,
    stEventAuth,
    rdEventAuth,
  );
  instructions.push(ix);

  // Step 3: Send transaction
  console.log("Sending transaction...\n");
  await sendTx(rpc, rpcSub, admin, instructions, "RedeemToken");

  console.log(
    `\nDone. Initiated redemption of ${amountShares} ${tokenName.toUpperCase()} shares.`
  );
  console.log(`RedemptionOperation PDA: ${redemptionOpAddr}`);
  console.log(`Salt: ${salt}`);
  console.log(
    "\nThe operation is now PENDING. An operator can execute it within 14 days,"
  );
  console.log("or anyone can cancel it after the deadline passes.");
}

main().catch((err) => {
  console.error("\nFailed:", err);
  process.exit(1);
});
