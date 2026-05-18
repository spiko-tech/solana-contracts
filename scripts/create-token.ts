/**
 * create-token.ts
 *
 * Creates a new Token-2022 token with all Spiko extensions and registers it
 * with the program suite. Assumes setup-programs has already been run.
 *
 * Usage:
 *   pnpm tsx scripts/create-token.ts \
 *     --cluster <localnet|devnet|mainnet-beta> \
 *     --keypair ./admin.json \
 *     --symbol EUTBL \
 *     --name "EU T-Bill Token" \
 *     --uri "https://spiko.finance/eutbl" \
 *     --decimals 6 \
 *     --minter-daily-limit 1000000 \
 *     --cg-daily-limit 500000
 */

import { Command, Options } from "@effect/cli";
import { NodeContext, NodeRuntime } from "@effect/platform-node";
import { Effect } from "effect";
import fs from "fs";
import path from "path";
import { type Address, address } from "@solana/kit";

import {
  getRpc,
  loadKeypair,
  accountExists,
  sendTx,
  getMinimumRent,
  generateKeyPairSigner,
  findHookConfigPda,
  findMinterConfigPda,
  findRedemptionConfigPda,
  findGatekeeperConfigPda,
  findRedemptionVaultAuthorityPda,
  findCgVaultAuthorityPda,
  findExtraAccountMetasPda,
  getRegisterMintInstructionAsync,
  getMtSetDailyLimitInstructionAsync,
  getCgSetDailyLimitInstructionAsync,
  TRANSFER_HOOK_PROGRAM_ADDRESS,
  TOKEN_2022_PROGRAM_ID,
  MINT_FIXED_EXTENSIONS_SIZE,
  getMintAccountSpace,
  buildCreateAccountInstruction,
  buildInitializeTransferHook,
  buildInitializePermanentDelegate,
  buildInitializeMetadataPointer,
  buildInitializeMint2,
  buildTokenMetadataInitialize,
  buildSetAuthority,
  getAssociatedTokenAddress,
  createAssociatedTokenAccountIdempotent,
} from "./lib/common.js";

// ── CLI Options ──────────────────────────────────────────────

const cluster = Options.choice("cluster", ["localnet", "devnet", "mainnet-beta"]);
const keypair = Options.text("keypair");
const symbol = Options.text("symbol");
const name = Options.text("name");
const uri = Options.text("uri");
const decimals = Options.integer("decimals");
const minterDailyLimit = Options.integer("minter-daily-limit");
const cgDailyLimit = Options.integer("cg-daily-limit");

// ── Command handler ──────────────────────────────────────────

async function run(args: {
  cluster: string;
  keypair: string;
  symbol: string;
  name: string;
  uri: string;
  decimals: number;
  minterDailyLimit: number;
  cgDailyLimit: number;
}) {
  console.log(`\n═══════════════════════════════════════════════════════`);
  console.log(`  Create Token — ${args.symbol} on ${args.cluster}`);
  console.log(`═══════════════════════════════════════════════════════\n`);

  const admin = await loadKeypair(args.keypair);
  const { rpc, rpcSub } = getRpc(args.cluster);

  console.log(`Admin: ${admin.address}`);

  // ── Verify programs are initialized ──────────────────────
  console.log(`\n── Verifying programs are initialized... ──`);

  const [hookConfigAddr] = await findHookConfigPda();
  const [minterConfigAddr] = await findMinterConfigPda();
  const [rdConfigAddr] = await findRedemptionConfigPda();
  const [cgConfigAddr] = await findGatekeeperConfigPda();

  const checks = [
    { name: "Transfer Hook", addr: hookConfigAddr },
    { name: "Minter", addr: minterConfigAddr },
    { name: "Redemption", addr: rdConfigAddr },
    { name: "Custodial Gatekeeper", addr: cgConfigAddr },
  ];

  for (const check of checks) {
    if (!(await accountExists(rpc, check.addr))) {
      console.error(`  ✗ ${check.name} not initialized. Run setup-programs first.`);
      process.exit(1);
    }
    console.log(`  ✓ ${check.name}`);
  }

  // ── Step 1: Create mint ──────────────────────────────────
  console.log(`\n── Creating mint... ──`);

  const mintKp = await generateKeyPairSigner();
  const mint = mintKp.address;
  console.log(`  Mint address: ${mint}`);

  const fullMintSize = getMintAccountSpace(args.name, args.symbol, args.uri);
  const mintRent = await getMinimumRent(rpc, fullMintSize);

  await sendTx(rpc, rpcSub, admin, [
    buildCreateAccountInstruction(admin, mintKp, mintRent, MINT_FIXED_EXTENSIONS_SIZE, TOKEN_2022_PROGRAM_ID),
    buildInitializeTransferHook(mint, admin.address, TRANSFER_HOOK_PROGRAM_ADDRESS as Address),
    buildInitializePermanentDelegate(mint, admin.address),
    buildInitializeMetadataPointer(mint, admin.address, mint),
    buildInitializeMint2(mint, args.decimals, admin.address),
  ], `Create mint ${args.symbol}`);

  // ── Step 2: Initialize metadata ──────────────────────────
  await sendTx(rpc, rpcSub, admin, [
    buildTokenMetadataInitialize(mint, admin.address, admin, args.name, args.symbol, args.uri),
  ], `Init metadata ${args.symbol}`);

  // ── Step 3: Register mint with Transfer Hook ─────────────
  const regMintIx = await getRegisterMintInstructionAsync({ admin, mint });
  await sendTx(rpc, rpcSub, admin, [regMintIx], `Register mint ${args.symbol}`);

  // ── Step 4: Transfer mint authority to Minter PDA ────────
  await sendTx(rpc, rpcSub, admin, [
    buildSetAuthority(mint, admin, 0 /* MintTokens */, minterConfigAddr),
  ], "Transfer mint authority to Minter Config PDA");

  // ── Step 5: Set minter daily limit ───────────────────────
  const minterLimit = BigInt(args.minterDailyLimit) * 10n ** BigInt(args.decimals);
  const mtLimitIx = await getMtSetDailyLimitInstructionAsync({
    admin,
    mint,
    payer: admin,
    limit: minterLimit,
  });
  await sendTx(rpc, rpcSub, admin, [mtLimitIx], `Set minter daily limit (${args.minterDailyLimit} tokens)`);

  // ── Step 6: Set CG daily limit ───────────────────────────
  const cgLimit = BigInt(args.cgDailyLimit) * 10n ** BigInt(args.decimals);
  const cgLimitIx = await getCgSetDailyLimitInstructionAsync({
    admin,
    mint,
    payer: admin,
    limit: cgLimit,
  });
  await sendTx(rpc, rpcSub, admin, [cgLimitIx], `Set CG daily limit (${args.cgDailyLimit} tokens)`);

  // ── Step 7: Create vault ATAs ────────────────────────────
  console.log(`\n── Creating vault ATAs... ──`);

  const [rdVaultAuth] = await findRedemptionVaultAuthorityPda();
  const [cgVaultAuth] = await findCgVaultAuthorityPda();

  const rdVault = await getAssociatedTokenAddress(rdVaultAuth, mint);
  const cgVault = await getAssociatedTokenAddress(cgVaultAuth, mint);

  await sendTx(rpc, rpcSub, admin, [
    createAssociatedTokenAccountIdempotent(admin, rdVault, rdVaultAuth, mint),
    createAssociatedTokenAccountIdempotent(admin, cgVault, cgVaultAuth, mint),
  ], "Create vault ATAs (Redemption + CG)");

  // ── Output ───────────────────────────────────────────────
  const [extraMetasAddr] = await findExtraAccountMetasPda({ mint });

  const output = {
    cluster: args.cluster,
    symbol: args.symbol,
    name: args.name,
    uri: args.uri,
    decimals: args.decimals,
    mint: mint,
    admin: admin.address,
    minterConfig: minterConfigAddr,
    minterDailyLimit: `${args.minterDailyLimit} tokens`,
    redemptionConfig: rdConfigAddr,
    redemptionVaultAuthority: rdVaultAuth,
    redemptionVault: rdVault,
    gatekeeperConfig: cgConfigAddr,
    cgVaultAuthority: cgVaultAuth,
    cgVault: cgVault,
    cgDailyLimit: `${args.cgDailyLimit} tokens`,
    hookConfig: hookConfigAddr,
    extraAccountMetas: extraMetasAddr,
    createdAt: new Date().toISOString(),
  };

  const outDir = path.resolve(process.cwd(), "deployments");
  if (!fs.existsSync(outDir)) fs.mkdirSync(outDir, { recursive: true });
  const outPath = path.join(outDir, `${args.symbol}-${args.cluster}.json`);
  fs.writeFileSync(outPath, JSON.stringify(output, null, 2));

  console.log(`\n═══════════════════════════════════════════════════════`);
  console.log(`  Token ${args.symbol} created! Output: ${outPath}`);
  console.log(`═══════════════════════════════════════════════════════\n`);
}

const command = Command.make(
  "create-token",
  { cluster, keypair, symbol, name, uri, decimals, minterDailyLimit, cgDailyLimit },
  (args) => Effect.promise(() => run(args))
);

// ── Run ──────────────────────────────────────────────────────

const cli = Command.run(command, { name: "create-token", version: "0.1.0" });
cli(process.argv).pipe(Effect.provide(NodeContext.layer), NodeRuntime.runMain);
