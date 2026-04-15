/**
 * Burn tokens from an account via the SpikoToken program.
 *
 * The caller must have ROLE_BURNER. The burn is performed using the mint
 * authority PDA as the authority (which has freeze authority and can burn
 * from any account in Token-2022).
 *
 * Prerequisites:
 *   - deploy.ts has been run (all PDAs initialized)
 *   - Caller has ROLE_BURNER (deployer wallet has all roles)
 *
 * Usage:
 *   npx tsx scripts/burn.ts <eutbl|ustbl> <SOURCE_WALLET_ADDRESS> <AMOUNT_IN_SHARES>
 *
 * Example:
 *   npx tsx scripts/burn.ts eutbl 3QpaUQVDFTKKK56pDRxmVcX1rMjEjXRcLECmeAaGs9Sx 50
 *   # Burns 50 shares (= 5,000,000 raw) from that wallet's ATA
 */

import { address } from "@solana/kit";

import {
  userPermissionsPda,
  tokenConfigPda,
  mintAuthorityPda,
  spikoTokenEventAuthorityPda,
} from "./lib/pda.js";
import { burnToken } from "./lib/instructions.js";
import {
  setup,
  sendTx,
  resolveMintAddress,
  getAssociatedTokenAddress,
  readMintDecimals,
} from "./lib/shared.js";

async function main() {
  const tokenName = process.argv[2];
  const sourceArg = process.argv[3];
  const amountArg = process.argv[4];

  if (!tokenName || !sourceArg || !amountArg) {
    console.error(
      "Usage: npx tsx scripts/burn.ts <eutbl|ustbl> <SOURCE_WALLET_ADDRESS> <AMOUNT_IN_SHARES>"
    );
    console.error("Example: npx tsx scripts/burn.ts eutbl <ADDRESS> 50");
    process.exit(1);
  }

  const sourceWallet = address(sourceArg);
  const amountShares = parseFloat(amountArg);

  const { rpc, rpcSub, admin } = await setup();

  // Resolve mint address and read decimals from on-chain mint account
  const mintAddr = await resolveMintAddress(tokenName);
  const decimals = await readMintDecimals(rpc, mintAddr);
  const rawAmount = BigInt(Math.round(amountShares * 10 ** decimals));

  console.log(`=== Burn Tokens ===\n`);
  console.log(`Token:   ${tokenName.toUpperCase()}`);
  console.log(`Source:  ${sourceWallet}`);
  console.log(`Amount:  ${amountShares} shares (raw: ${rawAmount})\n`);

  console.log(`Mint:           ${mintAddr}`);

  // Derive PDAs
  const [adminPermsAddr] = await userPermissionsPda(admin.address);
  const [tokenConfigAddr] = await tokenConfigPda(mintAddr);
  const [mintAuthAddr] = await mintAuthorityPda(mintAddr);
  const [stEventAuth] = await spikoTokenEventAuthorityPda();

  // Derive source's Token-2022 ATA
  const sourceAta = await getAssociatedTokenAddress(sourceWallet, mintAddr);

  console.log(`TokenConfig:    ${tokenConfigAddr}`);
  console.log(`MintAuthority:  ${mintAuthAddr}`);
  console.log(`CallerPerms:    ${adminPermsAddr}`);
  console.log(`SourceATA:      ${sourceAta}\n`);

  // Build burn instruction
  console.log("Building BurnToken instruction...");
  const ix = burnToken(
    admin,
    tokenConfigAddr,
    mintAddr,
    sourceAta,
    mintAuthAddr,
    adminPermsAddr,
    rawAmount,
    stEventAuth,
  );

  // Send transaction
  console.log("Sending transaction...\n");
  await sendTx(rpc, rpcSub, admin, [ix], "BurnToken");

  console.log(
    `\nDone. Burned ${amountShares} ${tokenName.toUpperCase()} shares from ${sourceWallet}'s ATA.`
  );
}

main().catch((err) => {
  console.error("\nFailed:", err);
  process.exit(1);
});
