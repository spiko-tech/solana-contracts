/**
 * setup-minter.ts
 *
 * First-time deployment and initialization of the Minter program.
 * If already initialized, prints the upgrade command and exits.
 *
 * Flow:
 *   1. anchor build
 *   2. solana program deploy
 *   3. Initialize MinterConfig (admin = deployer keypair, mint_initiator from CLI)
 *   4. nominate_admin → Squads vault PDA (two-step transfer)
 *   5. accept_admin via Squads vault transaction (completes admin transfer)
 *
 * Usage:
 *   pnpm tsx scripts/setup-minter.ts \
 *     --cluster devnet \
 *     --keypair ./deployer.json \
 *     --minter-admin-squad-account <multisig-account-pubkey> \
 *     --minter-admin-squad-vault <vault-pda-pubkey> \
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
  createNoopSigner,
} from '@solana/kit';
import { sendTx } from './lib/sendTx.js';
import { MINTER_PROGRAM_ADDRESS } from '../clients/ts/minter/src/generated/programs/index.js';
import { findMinterConfigPda } from '../clients/ts/minter/src/generated/pdas/index.js';
import { loadKeypair } from './lib/loadKeypair.js';
import { getInitializeInstructionAsync } from '../clients/ts/minter/src/generated/instructions/initialize.js';
import { getNominateAdminInstructionAsync } from '../clients/ts/minter/src/generated/instructions/nominateAdmin.js';
import { getAcceptAdminInstructionAsync, ACCEPT_ADMIN_DISCRIMINATOR } from '../clients/ts/minter/src/generated/instructions/acceptAdmin.js';
import { getRpc } from './lib/getRpc.js';
import { executeSquadsProposal } from './lib/executeSquadsProposal.js';
import * as multisig from '@sqds/multisig';
import { PublicKey } from '@solana/web3.js';

const accountExists = async (rpc: Rpc<SolanaRpcApi>, addr: Address): Promise<boolean> => {
  const { value } = await rpc.getAccountInfo(addr, { encoding: 'base64' }).send();
  return value !== null;
};

const CLUSTER_URLS: Record<string, string> = {
  devnet: 'https://api.devnet.solana.com',
  'mainnet-beta': 'https://api.mainnet-beta.solana.com',
};

const initializeAndTransferAdmin = async (
  rpc: Rpc<SolanaRpcApi>,
  rpcSub: RpcSubscriptions<SolanaRpcSubscriptionsApi>,
  admin: KeyPairSigner,
  mintInitiator: string,
  minterAdminSquadAccount: string,
  minterAdminSquadVault: string,
  minterConfigAddr: string,
  cluster: string,
  keypairPath: string,
) => {
  console.log(`\n── Initializing Minter Config... ──`);
  const initIx = await getInitializeInstructionAsync({
    admin,
    mintInitiator: address(mintInitiator),
  });
  await sendTx(rpc, rpcSub, admin, [initIx], 'Init Minter');
  console.log(`  Config PDA: ${minterConfigAddr}`);

  console.log(`\n── Nominating Squads vault as pending admin... ──`);
  const nominateIx = await getNominateAdminInstructionAsync({
    admin,
    newAdmin: address(minterAdminSquadVault),
  });
  await sendTx(rpc, rpcSub, admin, [nominateIx], `Nominate admin → ${minterAdminSquadVault}`);

  if (cluster === 'mainnet-beta') {
    console.log(`\n── Manual step required (mainnet) ──`);
    console.log(`  The Squads vault is now the PENDING admin.`);
    console.log(`  To complete the transfer, propose the following instruction via Squads UI:\n`);
    console.log(`  Program:  ${MINTER_PROGRAM_ADDRESS}`);
    console.log(`  Instruction: accept_admin`);
    console.log(`  Data (hex): ${Buffer.from(ACCEPT_ADMIN_DISCRIMINATOR).toString('hex')}`);
    console.log(`  Accounts:`);
    console.log(`    1. ${minterAdminSquadVault} (signer, readonly)  — new_admin (vault)`);
    console.log(`    2. ${minterConfigAddr} (writable)              — minter_config PDA`);
    console.log(`\n  Once threshold approvals are met, execute the vault transaction.`);
  } else {
    console.log(`\n── Accepting admin via Squads vault transaction... ──`);
    const vaultSigner = createNoopSigner(address(minterAdminSquadVault));
    const acceptIx = await getAcceptAdminInstructionAsync({
      newAdmin: vaultSigner,
    });
    const rpcUrl = CLUSTER_URLS[cluster];
    if (!rpcUrl) throw new Error(`Unknown cluster: ${cluster}`);
    await executeSquadsProposal({
      rpcUrl,
      keypairPath,
      multisigPubkey: minterAdminSquadAccount,
      vaultIndex: 0,
      instructions: [acceptIx as any],
      label: 'Accept Admin',
    });
  }
};

const saveDeployment = (
  cluster: string,
  admin: KeyPairSigner,
  minterConfigAddr: string,
  minterAdminSquadAccount: string,
  minterAdminSquadVault: string,
  mintInitiator: string,
): string => {
  const outDir = path.resolve(process.cwd(), 'deployments');
  if (!fs.existsSync(outDir)) fs.mkdirSync(outDir, { recursive: true });

  const output = {
    cluster,
    deployer: admin.address,
    program: MINTER_PROGRAM_ADDRESS,
    minterConfig: minterConfigAddr,
    minterAdminSquadAccount,
    minterAdminSquadVault,
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
  minterAdminSquadAccount,
  minterAdminSquadVault,
  mintInitiator,
}: {
  cluster: string;
  keypair: string;
  minterAdminSquadAccount: string;
  minterAdminSquadVault: string;
  mintInitiator: string;
}) => {
  console.log(`\n═══════════════════════════════════════════════════════`);
  console.log(`  Setup Minter — cluster: ${cluster}`);
  console.log(`═══════════════════════════════════════════════════════\n`);

  const multisigPda = new PublicKey(minterAdminSquadAccount);
  const [derivedVault] = multisig.getVaultPda({ multisigPda, index: 0 });
  if (derivedVault.toBase58() !== minterAdminSquadVault) {
    throw new Error(
      `Vault PDA mismatch!\n` +
        `  Derived from squad account: ${derivedVault.toBase58()}\n` +
        `  Provided --minter-admin-squad-vault: ${minterAdminSquadVault}\n` +
        `  These must match. Verify your inputs.`,
    );
  }

  const admin = await loadKeypair(keypair);
  console.log(`Deployer/Temp Admin:      ${admin.address}`);
  console.log(`Squad Account:            ${minterAdminSquadAccount}`);
  console.log(`Squad Vault (on-chain):   ${minterAdminSquadVault}`);
  console.log(`Mint Initiator:           ${mintInitiator}`);
  console.log(`  Vault PDA derivation verified.`);

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

  await initializeAndTransferAdmin(rpc, rpcSub, admin, mintInitiator, minterAdminSquadAccount, minterAdminSquadVault, minterConfigAddr, cluster, keypair);

  const outPath = saveDeployment(cluster, admin, minterConfigAddr, minterAdminSquadAccount, minterAdminSquadVault, mintInitiator);

  console.log(`\n═══════════════════════════════════════════════════════`);
  console.log(`  Setup complete! Output: ${outPath}`);
  if (cluster === 'mainnet-beta') {
    console.log(`  Admin nomination pending — complete via Squads UI.`);
  } else {
    console.log(`  Config admin transferred to Squads vault (two-step).`);
  }
  console.log(`  Upgrade authority remains with deployer keypair.`);
  console.log(`═══════════════════════════════════════════════════════\n`);
};

const {
  values: {
    cluster,
    keypair,
    'minter-admin-squad-account': minterAdminSquadAccount,
    'minter-admin-squad-vault': minterAdminSquadVault,
    'mint-initiator': mintInitiator,
  },
} = parseArgs({
  options: {
    cluster: { type: 'string', default: 'devnet' },
    keypair: { type: 'string' },
    'minter-admin-squad-account': { type: 'string' },
    'minter-admin-squad-vault': { type: 'string' },
    'mint-initiator': { type: 'string' },
  },
  strict: true,
});

if (!keypair || !minterAdminSquadAccount || !minterAdminSquadVault || !mintInitiator) {
  console.error('Usage: pnpm tsx scripts/setup-minter.ts \\');
  console.error('  --cluster devnet \\');
  console.error('  --keypair ./deployer.json \\');
  console.error('  --minter-admin-squad-account <multisig-account-pubkey> \\');
  console.error('  --minter-admin-squad-vault <vault-pda-pubkey> \\');
  console.error('  --mint-initiator <pubkey>');
  process.exit(1);
}

main({ cluster, keypair, minterAdminSquadAccount, minterAdminSquadVault, mintInitiator }).catch((e) => {
  console.error(e);
  process.exit(1);
});
