/**
 * Mint tokens to a whitelisted recipient via the Minter program.
 *
 * Prerequisites:
 *   - deploy.ts has been run (all PDAs initialized)
 *   - MinterConfig PDA has been granted ROLE_MINTER (Phase 7 of deploy.ts)
 *   - Recipient has been whitelisted (whitelist.ts)
 *
 * Usage:
 *   npx tsx scripts/mint.ts <eutbl|ustbl> <RECIPIENT_ADDRESS> <AMOUNT_IN_SHARES> [SALT]
 *
 * Example:
 *   npx tsx scripts/mint.ts eutbl 3QpaUQVDFTKKK56pDRxmVcX1rMjEjXRcLECmeAaGs9Sx 100
 *   # Mints 100 shares (= 10,000,000 in raw amount with 5 decimals)
 */

import { address } from "@solana/kit";

import {
  userPermissionsPda,
  tokenConfigPda,
  mintAuthorityPda,
  minterConfigPda,
  dailyLimitPda,
  mintOperationPda,
  spikoTokenEventAuthorityPda,
  minterEventAuthorityPda,
} from "./lib/pda.js";
import { initiateMint } from "./lib/instructions.js";
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
  const recipientArg = process.argv[3];
  const amountArg = process.argv[4];
  const saltArg = process.argv[5];

  if (!tokenName || !recipientArg || !amountArg) {
    console.error(
      "Usage: npx tsx scripts/mint.ts <eutbl|ustbl> <RECIPIENT_ADDRESS> <AMOUNT_IN_SHARES> [SALT]"
    );
    console.error("Example: npx tsx scripts/mint.ts eutbl <ADDRESS> 100");
    process.exit(1);
  }

  const recipientAddr = address(recipientArg);
  const amountShares = parseFloat(amountArg);
  const salt = saltArg
    ? BigInt(saltArg)
    : BigInt(Math.floor(Math.random() * Number.MAX_SAFE_INTEGER));

  const { rpc, rpcSub, admin } = await setup();

  // Resolve mint address and read decimals from on-chain mint account
  const mintAddr = await resolveMintAddress(tokenName);
  const decimals = await readMintDecimals(rpc, mintAddr);
  const rawAmount = BigInt(Math.round(amountShares * 10 ** decimals));

  console.log(`=== Mint Tokens ===\n`);
  console.log(`Token:     ${tokenName.toUpperCase()}`);
  console.log(`Recipient: ${recipientAddr}`);
  console.log(`Amount:    ${amountShares} shares (raw: ${rawAmount})`);
  console.log(`Salt:      ${salt}\n`);

  console.log(`Mint:           ${mintAddr}`);

  // Derive all PDAs
  const [adminPermsAddr] = await userPermissionsPda(admin.address);
  const [minterConfigAddr] = await minterConfigPda();
  const [minterConfigPermsAddr] = await userPermissionsPda(minterConfigAddr);
  const [tokenConfigAddr] = await tokenConfigPda(mintAddr);
  const [mintAuthAddr] = await mintAuthorityPda(mintAddr);
  const [dailyLimitAddr] = await dailyLimitPda(mintAddr);
  const [recipientPermsAddr] = await userPermissionsPda(recipientAddr);
  const [mtEventAuth] = await minterEventAuthorityPda();
  const [stEventAuth] = await spikoTokenEventAuthorityPda();

  // Compute operation_id for MintOperation PDA
  const operationId = await computeOperationId(
    recipientAddr,
    mintAddr,
    rawAmount,
    salt
  );
  const [mintOpAddr] = await mintOperationPda(operationId);

  // Derive recipient's Token-2022 ATA
  const recipientAta = await getAssociatedTokenAddress(recipientAddr, mintAddr);

  console.log(`TokenConfig:    ${tokenConfigAddr}`);
  console.log(`MintAuthority:  ${mintAuthAddr}`);
  console.log(`MinterConfig:   ${minterConfigAddr}`);
  console.log(`MinterPerms:    ${minterConfigPermsAddr}`);
  console.log(`DailyLimit:     ${dailyLimitAddr}`);
  console.log(`MintOperation:  ${mintOpAddr}`);
  console.log(`RecipientPerms: ${recipientPermsAddr}`);
  console.log(`RecipientATA:   ${recipientAta}\n`);

  // Step 1: Create recipient's ATA if it doesn't exist
  const instructions: any[] = [];

  if (!(await accountExists(rpc, recipientAta))) {
    console.log("Creating recipient's Associated Token Account...");
    const createAtaIx = createAssociatedTokenAccountIdempotent(
      admin,
      recipientAta,
      recipientAddr,
      mintAddr
    );
    instructions.push(createAtaIx);
  } else {
    console.log("Recipient ATA already exists.");
  }

  // Step 2: Build InitiateMint instruction
  console.log("Building InitiateMint instruction...");
  const mintIx = initiateMint(
    admin,
    minterConfigAddr,
    dailyLimitAddr,
    mintOpAddr,
    adminPermsAddr,
    tokenConfigAddr,
    mintAddr,
    recipientAta,
    mintAuthAddr,
    minterConfigPermsAddr,
    recipientPermsAddr,
    recipientAddr,
    rawAmount,
    salt,
    stEventAuth,
    mtEventAuth,
  );
  instructions.push(mintIx);

  // Step 3: Send transaction
  console.log("Sending transaction...\n");
  await sendTx(rpc, rpcSub, admin, instructions, "InitiateMint");

  console.log(
    `\nDone. Minted ${amountShares} ${tokenName.toUpperCase()} shares to ${recipientAddr}.`
  );
  console.log(
    "(If over daily limit, a PENDING MintOperation was created instead.)"
  );
}

main().catch((err) => {
  console.error("\nFailed:", err);
  process.exit(1);
});
