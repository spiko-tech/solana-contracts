/**
 * Execute a pending redemption operation via the Redemption program.
 *
 * Burns the tokens held in the vault. The operator must have ROLE_REDEMPTION_EXECUTOR.
 * The vault authority PDA must have ROLE_BURNER (granted in deploy.ts Phase 8).
 *
 * CPI chain: redemption::execute_redemption -> spiko_token::burn -> Token-2022::Burn
 *
 * Prerequisites:
 *   - deploy.ts has been run (including Phase 8 — vault authority roles)
 *   - A redeem.ts call was made previously, creating a PENDING RedemptionOperation
 *   - Operator has ROLE_REDEMPTION_EXECUTOR
 *   - The deadline has NOT passed yet
 *
 * Usage:
 *   npx tsx scripts/execute-redemption.ts <eutbl|ustbl> <USER_ADDRESS> <AMOUNT_IN_SHARES> <SALT>
 *
 * Example:
 *   npx tsx scripts/execute-redemption.ts eutbl 3QpaUQVDFTKKK56pDRxmVcX1rMjEjXRcLECmeAaGs9Sx 10 12345
 */

import { address } from "@solana/kit";

import {
  userPermissionsPda,
  tokenConfigPda,
  mintAuthorityPda,
  redemptionConfigPda,
  vaultAuthorityPda,
  redemptionOperationPda,
} from "./lib/pda.js";
import { executeRedemption } from "./lib/instructions.js";
import {
  setup,
  sendTx,
  resolveMintAddress,
  getAssociatedTokenAddress,
  computeOperationId,
  readMintDecimals,
} from "./lib/shared.js";

async function main() {
  const tokenName = process.argv[2];
  const userArg = process.argv[3];
  const amountArg = process.argv[4];
  const saltArg = process.argv[5];

  if (!tokenName || !userArg || !amountArg || !saltArg) {
    console.error(
      "Usage: npx tsx scripts/execute-redemption.ts <eutbl|ustbl> <USER_ADDRESS> <AMOUNT_IN_SHARES> <SALT>"
    );
    console.error(
      "Example: npx tsx scripts/execute-redemption.ts eutbl <ADDRESS> 10 12345"
    );
    process.exit(1);
  }

  const userAddr = address(userArg);
  const amountShares = parseFloat(amountArg);
  const salt = BigInt(saltArg);

  const { rpc, rpcSub, admin } = await setup();

  // Admin is the operator for devnet testing
  const operator = admin;

  // Resolve mint address and read decimals from on-chain mint account
  const mintAddr = await resolveMintAddress(tokenName);
  const decimals = await readMintDecimals(rpc, mintAddr);
  const rawAmount = BigInt(Math.round(amountShares * 10 ** decimals));

  console.log(`=== Execute Redemption ===\n`);
  console.log(`Token:   ${tokenName.toUpperCase()}`);
  console.log(`User:    ${userAddr}`);
  console.log(`Amount:  ${amountShares} shares (raw: ${rawAmount})`);
  console.log(`Salt:    ${salt}\n`);

  console.log(`Mint:              ${mintAddr}`);

  // Derive all PDAs
  const [operatorPermsAddr] = await userPermissionsPda(operator.address);
  const [tokenConfigAddr] = await tokenConfigPda(mintAddr);
  const [mintAuthAddr] = await mintAuthorityPda(mintAddr);
  const [redemptionConfigAddr] = await redemptionConfigPda();
  const [vaultAuthAddr] = await vaultAuthorityPda();
  const [vaultAuthPermsAddr] = await userPermissionsPda(vaultAuthAddr);

  // Compute operation_id for RedemptionOperation PDA
  const operationId = await computeOperationId(
    userAddr,
    mintAddr,
    rawAmount,
    salt
  );
  const [redemptionOpAddr] = await redemptionOperationPda(operationId);

  // Derive vault ATA
  const vaultAta = await getAssociatedTokenAddress(vaultAuthAddr, mintAddr);

  console.log(`TokenConfig:       ${tokenConfigAddr}`);
  console.log(`MintAuthority:     ${mintAuthAddr}`);
  console.log(`OperatorPerms:     ${operatorPermsAddr}`);
  console.log(`RedemptionConfig:  ${redemptionConfigAddr}`);
  console.log(`VaultAuthority:    ${vaultAuthAddr}`);
  console.log(`VaultAuthPerms:    ${vaultAuthPermsAddr}`);
  console.log(`RedemptionOp:      ${redemptionOpAddr}`);
  console.log(`VaultATA:          ${vaultAta}\n`);

  // Build ExecuteRedemption instruction
  console.log("Building ExecuteRedemption instruction...");
  const ix = executeRedemption(
    operator,
    redemptionConfigAddr,
    redemptionOpAddr,
    operatorPermsAddr,
    tokenConfigAddr,
    mintAddr,
    vaultAta,
    mintAuthAddr,
    vaultAuthAddr,
    vaultAuthPermsAddr,
    userAddr,
    rawAmount,
    salt
  );

  // Send transaction
  console.log("Sending transaction...\n");
  await sendTx(rpc, rpcSub, admin, [ix], "ExecuteRedemption");

  console.log(
    `\nDone. Executed redemption of ${amountShares} ${tokenName.toUpperCase()} shares.`
  );
  console.log("Tokens have been burned from the vault.");
}

main().catch((err) => {
  console.error("\nFailed:", err);
  process.exit(1);
});
