/**
 * setup-programs.ts
 *
 * Deploys (optionally) and initializes all 4 Spiko programs.
 * Idempotent — safe to re-run.
 *
 * Usage:
 *   pnpm tsx scripts/setup-programs.ts \
 *     --cluster <localnet|devnet|mainnet-beta> \
 *     --keypair ./admin.json \
 *     --deploy <true|false> \
 *     --whitelist-authority <pubkey> \
 *     --mint-initiator <pubkey> \
 *     --redemption-authority <pubkey> \
 *     --gatekeeper-initiator <pubkey>
 */

import { Command, Options } from "@effect/cli";
import { NodeContext, NodeRuntime } from "@effect/platform-node";
import { Effect, Console } from "effect";
import { execSync } from "child_process";
import fs from "fs";
import path from "path";
import { type Address, address } from "@solana/kit";

import {
  getRpc,
  loadKeypair,
  accountExists,
  sendTx,
  findHookConfigPda,
  findMinterConfigPda,
  findRedemptionConfigPda,
  findGatekeeperConfigPda,
  findRedemptionVaultAuthorityPda,
  findCgVaultAuthorityPda,
  findWhitelistStatePda,
  getThInitializeInstructionAsync,
  getMtInitializeInstructionAsync,
  getRdInitializeInstructionAsync,
  getCgInitializeInstructionAsync,
  getAddGateInstructionAsync,
  TRANSFER_HOOK_PROGRAM_ADDRESS,
  MINTER_PROGRAM_ADDRESS,
  REDEMPTION_PROGRAM_ADDRESS,
  CUSTODIAL_GATEKEEPER_PROGRAM_ADDRESS,
} from "./lib/common.js";

// ── CLI Options ──────────────────────────────────────────────

const cluster = Options.choice("cluster", ["localnet", "devnet", "mainnet-beta"]);
const keypair = Options.text("keypair");
const deploy = Options.choice("deploy", ["true", "false"]);
const whitelistAuthority = Options.text("whitelist-authority");
const mintInitiator = Options.text("mint-initiator");
const redemptionAuthority = Options.text("redemption-authority");
const gatekeeperInitiator = Options.text("gatekeeper-initiator");

// ── Program deploy info ──────────────────────────────────────

const PROGRAMS = [
  { name: "transfer_hook", id: TRANSFER_HOOK_PROGRAM_ADDRESS },
  { name: "minter", id: MINTER_PROGRAM_ADDRESS },
  { name: "redemption", id: REDEMPTION_PROGRAM_ADDRESS },
  { name: "custodial_gatekeeper", id: CUSTODIAL_GATEKEEPER_PROGRAM_ADDRESS },
] as const;

// ── Command handler ──────────────────────────────────────────

async function run(args: {
  cluster: string;
  keypair: string;
  deploy: string;
  whitelistAuthority: string;
  mintInitiator: string;
  redemptionAuthority: string;
  gatekeeperInitiator: string;
}) {
  console.log(`\n═══════════════════════════════════════════════════════`);
  console.log(`  Setup Programs — cluster: ${args.cluster}`);
  console.log(`═══════════════════════════════════════════════════════\n`);

  const admin = await loadKeypair(args.keypair);
  console.log(`Admin: ${admin.address}`);

  const { rpc, rpcSub } = getRpc(args.cluster);

  // ── Step 1: Deploy (optional) ────────────────────────────
  if (args.deploy === "true") {
    console.log(`\n── Building programs... ──`);
    execSync("anchor build", { stdio: "inherit", cwd: process.cwd() });

    console.log(`\n── Deploying programs... ──`);
    const clusterFlag =
      args.cluster === "localnet" ? "-u localhost" :
      args.cluster === "devnet" ? "-u devnet" :
      "-u mainnet-beta";

    for (const prog of PROGRAMS) {
      const soPath = `target/deploy/${prog.name}.so`;
      const keypairPath = `target/deploy/${prog.name}-keypair.json`;
      if (!fs.existsSync(soPath)) {
        console.log(`  ✗ ${soPath} not found, skipping`);
        continue;
      }
      console.log(`  Deploying ${prog.name}...`);
      execSync(
        `solana program deploy ${soPath} --program-id ${keypairPath} ${clusterFlag}`,
        { stdio: "inherit", cwd: process.cwd() }
      );
    }
  } else {
    console.log(`Skipping deploy (--deploy false)`);
  }

  // ── Step 2: Init Transfer Hook ───────────────────────────
  console.log(`\n── Initializing programs... ──`);

  const [hookConfigAddr] = await findHookConfigPda();
  if (!(await accountExists(rpc, hookConfigAddr))) {
    const ix = await getThInitializeInstructionAsync({
      admin,
      whitelistAuthority: address(args.whitelistAuthority as Address),
    });
    await sendTx(rpc, rpcSub, admin, [ix], "Init Transfer Hook");
  } else {
    console.log(`  Transfer Hook already initialized, skipping`);
  }

  // ── Step 3: Init Minter ──────────────────────────────────
  const [minterConfigAddr] = await findMinterConfigPda();
  if (!(await accountExists(rpc, minterConfigAddr))) {
    const ix = await getMtInitializeInstructionAsync({
      admin,
      mintInitiator: address(args.mintInitiator as Address),
    });
    await sendTx(rpc, rpcSub, admin, [ix], "Init Minter");
  } else {
    console.log(`  Minter already initialized, skipping`);
  }

  // ── Step 4: Init Redemption ──────────────────────────────
  const [rdConfigAddr] = await findRedemptionConfigPda();
  if (!(await accountExists(rpc, rdConfigAddr))) {
    const ix = await getRdInitializeInstructionAsync({
      admin,
      redemptionAuthority: address(args.redemptionAuthority as Address),
    });
    await sendTx(rpc, rpcSub, admin, [ix], "Init Redemption");
  } else {
    console.log(`  Redemption already initialized, skipping`);
  }

  // ── Step 5: Init Custodial Gatekeeper ────────────────────
  const [cgConfigAddr] = await findGatekeeperConfigPda();
  if (!(await accountExists(rpc, cgConfigAddr))) {
    const ix = await getCgInitializeInstructionAsync({
      admin,
      gatekeeperInitiator: address(args.gatekeeperInitiator as Address),
    });
    await sendTx(rpc, rpcSub, admin, [ix], "Init Custodial Gatekeeper");
  } else {
    console.log(`  Custodial Gatekeeper already initialized, skipping`);
  }

  // ── Step 6: Add gate for vault PDAs ──────────────────────
  console.log(`\n── Gating vault PDAs... ──`);

  const [rdVaultAuth] = await findRedemptionVaultAuthorityPda();
  const [rdVaultWlPda] = await findWhitelistStatePda({ wallet: rdVaultAuth });
  if (!(await accountExists(rpc, rdVaultWlPda))) {
    const ix = await getAddGateInstructionAsync({ admin, wallet: rdVaultAuth, payer: admin });
    await sendTx(rpc, rpcSub, admin, [ix], "Gate Redemption vault (WHITELISTED_GATE)");
  } else {
    console.log(`  Redemption vault already gated, skipping`);
  }

  const [cgVaultAuth] = await findCgVaultAuthorityPda();
  const [cgVaultWlPda] = await findWhitelistStatePda({ wallet: cgVaultAuth });
  if (!(await accountExists(rpc, cgVaultWlPda))) {
    const ix = await getAddGateInstructionAsync({ admin, wallet: cgVaultAuth, payer: admin });
    await sendTx(rpc, rpcSub, admin, [ix], "Gate CG vault (WHITELISTED_GATE)");
  } else {
    console.log(`  CG vault already gated, skipping`);
  }

  // ── Output ───────────────────────────────────────────────
  const output = {
    cluster: args.cluster,
    admin: admin.address,
    programs: {
      transferHook: TRANSFER_HOOK_PROGRAM_ADDRESS,
      minter: MINTER_PROGRAM_ADDRESS,
      redemption: REDEMPTION_PROGRAM_ADDRESS,
      custodialGatekeeper: CUSTODIAL_GATEKEEPER_PROGRAM_ADDRESS,
    },
    pdas: {
      hookConfig: hookConfigAddr,
      minterConfig: minterConfigAddr,
      redemptionConfig: rdConfigAddr,
      redemptionVaultAuthority: rdVaultAuth,
      gatekeeperConfig: cgConfigAddr,
      cgVaultAuthority: cgVaultAuth,
    },
    authorities: {
      whitelistAuthority: args.whitelistAuthority,
      mintInitiator: args.mintInitiator,
      redemptionAuthority: args.redemptionAuthority,
      gatekeeperInitiator: args.gatekeeperInitiator,
    },
    setupAt: new Date().toISOString(),
  };

  const outDir = path.resolve(process.cwd(), "deployments");
  if (!fs.existsSync(outDir)) fs.mkdirSync(outDir, { recursive: true });
  const outPath = path.join(outDir, `programs-${args.cluster}.json`);
  fs.writeFileSync(outPath, JSON.stringify(output, null, 2));

  console.log(`\n═══════════════════════════════════════════════════════`);
  console.log(`  Setup complete! Output: ${outPath}`);
  console.log(`═══════════════════════════════════════════════════════\n`);
}

const command = Command.make(
  "setup-programs",
  { cluster, keypair, deploy, whitelistAuthority, mintInitiator, redemptionAuthority, gatekeeperInitiator },
  (args) => Effect.promise(() => run(args))
);

// ── Run ──────────────────────────────────────────────────────

const cli = Command.run(command, { name: "setup-programs", version: "0.1.0" });
cli(process.argv).pipe(Effect.provide(NodeContext.layer), NodeRuntime.runMain);
