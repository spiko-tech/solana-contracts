/**
 * setup-minter.ts
 *
 * First-time deployment and initialization of the Minter program.
 * If already initialized, prints the upgrade command and exits.
 *
 * Flow:
 *   1. anchor build
 *   2. solana program deploy
 *   3. Initialize MinterConfig (admin = keypair, mint_initiator from CLI)
 *   4. set_admin → --minter-admin (multisig)
 *
 * Usage:
 *   pnpm tsx scripts/setup-minter.ts \
 *     --cluster devnet \
 *     --keypair ./deployer.json \
 *     --minter-admin <multisig-pubkey> \
 *     --mint-initiator <pubkey>
 */
import { parseArgs } from 'node:util';
import { execFileSync } from 'node:child_process';
import fs from 'fs';
import path from 'path';
import {
  type KeyPairSigner,
  type Rpc,
  type SolanaRpcApi,
  type RpcSubscriptions,
  type SolanaRpcSubscriptionsApi,
  type Address,
  address,
} from '@solana/kit';
import { sendTx } from './lib/sendTx.js';
import { MINTER_PROGRAM_ADDRESS } from '../clients/ts/minter/src/generated/programs/index.js';
import { findMinterConfigPda } from '../clients/ts/minter/src/generated/pdas/index.js';
import { loadKeypair } from './lib/loadKeypair.js';
import { getInitializeInstructionAsync } from '../clients/ts/minter/src/generated/instructions/initialize.js';
import { getSetAdminInstructionAsync } from '../clients/ts/minter/src/generated/instructions/setAdmin.js';
import { getRpc } from './lib/getRpc.js';

const accountExists = async (rpc: Rpc<SolanaRpcApi>, addr: Address): Promise<boolean> => {
  const { value } = await rpc.getAccountInfo(addr, { encoding: 'base64' }).send();
  return value !== null;
};

const initializeAndTransferAdmin = async (
  rpc: Rpc<SolanaRpcApi>,
  rpcSub: RpcSubscriptions<SolanaRpcSubscriptionsApi>,
  admin: KeyPairSigner,
  mintInitiator: string,
  minterAdmin: string,
  minterConfigAddr: string,
) => {
  console.log(`\n── Initializing Minter Config... ──`);
  const initIx = await getInitializeInstructionAsync({
    admin,
    mintInitiator: address(mintInitiator),
  });
  await sendTx(rpc, rpcSub, admin, [initIx], 'Init Minter');
  console.log(`  Config PDA: ${minterConfigAddr}`);

  console.log(`\n── Transferring config admin to multisig... ──`);
  const setAdminIx = await getSetAdminInstructionAsync({
    admin,
    newAdmin: address(minterAdmin),
  });
  await sendTx(rpc, rpcSub, admin, [setAdminIx], `Set admin → ${minterAdmin}`);
};

const saveDeployment = (
  cluster: string,
  admin: KeyPairSigner,
  minterConfigAddr: string,
  minterAdmin: string,
  mintInitiator: string,
): string => {
  const outDir = path.resolve(process.cwd(), 'deployments');
  if (!fs.existsSync(outDir)) fs.mkdirSync(outDir, { recursive: true });

  const output = {
    cluster,
    deployer: admin.address,
    program: MINTER_PROGRAM_ADDRESS,
    minterConfig: minterConfigAddr,
    minterAdmin,
    mintInitiator,
    upgradeAuthority: admin.address,
    setupAt: new Date().toISOString(),
  };

  const outPath = path.join(outDir, `minter-${cluster}.json`);
  fs.writeFileSync(outPath, JSON.stringify(output, null, 2));
  return outPath;
};

const main = async ({
  cluster,
  keypair,
  minterAdmin,
  mintInitiator,
}: {
  cluster: string;
  keypair: string;
  minterAdmin: string;
  mintInitiator: string;
}) => {
  console.log(`\n═══════════════════════════════════════════════════════`);
  console.log(`  Setup Minter — cluster: ${cluster}`);
  console.log(`═══════════════════════════════════════════════════════\n`);

  const admin = await loadKeypair(keypair);
  console.log(`Deployer/Temp Admin: ${admin.address}`);
  console.log(`Final Admin:         ${minterAdmin}`);
  console.log(`Mint Initiator:      ${mintInitiator}`);

  const { rpc, rpcSub } = getRpc(cluster);
  const [minterConfigAddr] = await findMinterConfigPda();
  const exists = await accountExists(rpc, minterConfigAddr);

  if (exists) {
    console.error(`\n✗ Minter already initialized (config: ${minterConfigAddr}).`);
    console.error(`\nTo upgrade the program, run:`);
    console.error(`  solana program deploy target/deploy/minter.so \\`);
    console.error(`    --program-id ${MINTER_PROGRAM_ADDRESS} \\`);
    console.error(`    -u ${cluster} \\`);
    console.error(`    -k <upgrade-authority-keypair>`);
    console.error(`\nOr propose via Squads if upgrade authority is a multisig.\n`);
    process.exit(1);
  }

  console.log(`\n── Building minter program... ──`);
  execFileSync('anchor', ['build', '-p', 'minter', '--ignore-keys'], { stdio: 'inherit' });
  console.log(`  ✓ Build successful`);

  console.log(`\n── Deploying minter program... ──`);
  const soPath = 'target/deploy/minter.so';
  if (!fs.existsSync(soPath)) {
    throw new Error(`${soPath} not found`);
  }
  execFileSync('solana', ['program', 'deploy', soPath, '--program-id', 'target/deploy/minter-keypair.json', '-u', cluster, '-k', keypair], {
    stdio: 'inherit',
  });
  console.log(`  ✓ Program deployed: ${MINTER_PROGRAM_ADDRESS}`);

  await initializeAndTransferAdmin(rpc, rpcSub, admin, mintInitiator, minterAdmin, minterConfigAddr);

  const outPath = saveDeployment(cluster, admin, minterConfigAddr, minterAdmin, mintInitiator);

  console.log(`\n═══════════════════════════════════════════════════════`);
  console.log(`  Setup complete! Output: ${outPath}`);
  console.log(`  Config admin transferred to multisig.`);
  console.log(`  Upgrade authority remains with deployer keypair.`);
  console.log(`═══════════════════════════════════════════════════════\n`);
};

const {
  values: { cluster, keypair, 'minter-admin': minterAdmin, 'mint-initiator': mintInitiator },
} = parseArgs({
  options: {
    cluster: { type: 'string', default: 'devnet' },
    keypair: { type: 'string' },
    'minter-admin': { type: 'string' },
    'mint-initiator': { type: 'string' },
  },
  strict: true,
});

if (!keypair || !minterAdmin || !mintInitiator) {
  console.error('Usage: pnpm tsx scripts/setup-minter.ts \\');
  console.error('  --cluster devnet \\');
  console.error('  --keypair ./deployer.json \\');
  console.error('  --minter-admin <multisig-pubkey> \\');
  console.error('  --mint-initiator <pubkey>');
  process.exit(1);
}

main({ cluster, keypair, minterAdmin, mintInitiator }).catch((e) => {
  console.error(e);
  process.exit(1);
});
