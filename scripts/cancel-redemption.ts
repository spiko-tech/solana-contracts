/**
 * Cancel a redemption after the deadline has passed.
 *
 * Anyone can call this — no permission check. Tokens are refunded from the
 * vault back to the user's token account.
 *
 * Note: This bypasses the spiko_token pause check by using a direct
 * Token-2022 TransferChecked CPI (vault authority PDA signs). This is
 * intentional — users should always be able to recover their funds.
 *
 * Prerequisites:
 *   - A redeem.ts call was made previously, creating a PENDING RedemptionOperation
 *   - The 14-day deadline HAS passed
 *   - Status is still PENDING (not already EXECUTED)
 *
 * Usage:
 *   npx tsx scripts/cancel-redemption.ts <eutbl|ustbl> <USER_ADDRESS> <AMOUNT_IN_SHARES> <SALT>
 *
 * Example:
 *   npx tsx scripts/cancel-redemption.ts eutbl 3QpaUQVDFTKKK56pDRxmVcX1rMjEjXRcLECmeAaGs9Sx 10 12345
 */

import { address } from "@solana/kit";

import { TOKEN_DECIMALS } from "./lib/constants.js";
import {
  userPermissionsPda,
  tokenConfigPda,
  extraAccountMetaListPda,
  redemptionConfigPda,
  vaultAuthorityPda,
  redemptionOperationPda,
} from "./lib/pda.js";
import { cancelRedemption } from "./lib/instructions.js";
import {
  setup,
  sendTx,
  resolveMintAddress,
  getAssociatedTokenAddress,
  computeOperationId,
} from "./lib/shared.js";

async function main() {
  const tokenName = process.argv[2];
  const userArg = process.argv[3];
  const amountArg = process.argv[4];
  const saltArg = process.argv[5];

  if (!tokenName || !userArg || !amountArg || !saltArg) {
    console.error(
      "Usage: npx tsx scripts/cancel-redemption.ts <eutbl|ustbl> <USER_ADDRESS> <AMOUNT_IN_SHARES> <SALT>"
    );
    console.error(
      "Example: npx tsx scripts/cancel-redemption.ts eutbl <ADDRESS> 10 12345"
    );
    process.exit(1);
  }

  const userAddr = address(userArg);
  const amountShares = parseFloat(amountArg);
  const rawAmount = BigInt(Math.round(amountShares * 10 ** TOKEN_DECIMALS));
  const salt = BigInt(saltArg);

  console.log(`=== Cancel Redemption ===\n`);
  console.log(`Token:   ${tokenName.toUpperCase()}`);
  console.log(`User:    ${userAddr}`);
  console.log(`Amount:  ${amountShares} shares (raw: ${rawAmount})`);
  console.log(`Salt:    ${salt}\n`);

  const { rpc, rpcSub, admin } = await setup();

  // Resolve mint address
  const mintAddr = await resolveMintAddress(tokenName);
  console.log(`Mint:              ${mintAddr}`);

  // Derive all PDAs
  const [redemptionConfigAddr] = await redemptionConfigPda();
  const [vaultAuthAddr] = await vaultAuthorityPda();
  const [tokenConfigAddr] = await tokenConfigPda(mintAddr);
  const [extraMetaListAddr] = await extraAccountMetaListPda(mintAddr);
  const [vaultAuthPermsAddr] = await userPermissionsPda(vaultAuthAddr);
  const [userPermsAddr] = await userPermissionsPda(userAddr);

  // Compute operation_id for RedemptionOperation PDA
  const operationId = await computeOperationId(
    userAddr,
    mintAddr,
    rawAmount,
    salt
  );
  const [redemptionOpAddr] = await redemptionOperationPda(operationId);

  // Derive ATAs
  const vaultAta = await getAssociatedTokenAddress(vaultAuthAddr, mintAddr);
  const userAta = await getAssociatedTokenAddress(userAddr, mintAddr);

  console.log(`RedemptionConfig:  ${redemptionConfigAddr}`);
  console.log(`VaultAuthority:    ${vaultAuthAddr}`);
  console.log(`TokenConfig:       ${tokenConfigAddr}`);
  console.log(`ExtraMetaList:     ${extraMetaListAddr}`);
  console.log(`VaultAuthPerms:    ${vaultAuthPermsAddr}`);
  console.log(`UserPerms:         ${userPermsAddr}`);
  console.log(`RedemptionOp:      ${redemptionOpAddr}`);
  console.log(`VaultATA:          ${vaultAta}`);
  console.log(`UserATA:           ${userAta}\n`);

  // Build CancelRedemption instruction
  console.log("Building CancelRedemption instruction...");
  const ix = cancelRedemption(
    admin, // anyone can call — using admin as the signer/payer
    redemptionConfigAddr,
    redemptionOpAddr,
    vaultAta,
    userAta,
    vaultAuthAddr,
    mintAddr,
    extraMetaListAddr,
    tokenConfigAddr,
    vaultAuthPermsAddr,
    userPermsAddr,
    userAddr,
    rawAmount,
    salt
  );

  // Send transaction
  console.log("Sending transaction...\n");
  await sendTx(rpc, rpcSub, admin, [ix], "CancelRedemption");

  console.log(
    `\nDone. Canceled redemption of ${amountShares} ${tokenName.toUpperCase()} shares.`
  );
  console.log(`Tokens have been refunded to ${userAddr}'s ATA.`);
}

main().catch((err) => {
  console.error("\nFailed:", err);
  process.exit(1);
});
