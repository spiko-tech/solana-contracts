/**
 * setup-acl.ts
 *
 * One-time creation of the shared ABL Gate allow/block lists.
 * The gate-authority (a Squads multisig vault) becomes the immutable owner.
 *
 * This script derives the list PDAs, prints raw instruction details,
 * and on devnet executes the proposal via the Squads SDK directly.
 *
 * Usage:
 *   pnpm tsx scripts/setup-acl.ts \
 *     --cluster devnet \
 *     --gate-authority <squads-vault-pubkey> \
 *     --keypair ./deployer.json \
 *     --multisig-pubkey <multisig-pubkey> \
 *     --vault-index 0
 */

import { parseArgs } from 'node:util';
import fs from 'fs';
import path from 'path';
import { address } from '@solana/kit';
import { getCreateListInstruction, findListConfigPda, TOKEN_ACL_GATE_PROGRAM_PROGRAM_ADDRESS, Mode } from '@solana/token-acl-gate-sdk';
import { printInstruction } from './lib/printInstruction.js';
import { executeSquadsProposal } from './lib/executeSquadsProposal.js';
import { getRpc } from './lib/getRpc.js';

const CLUSTER_URLS: Record<string, string> = {
  devnet: 'https://api.devnet.solana.com',
  'mainnet-beta': 'https://api.mainnet-beta.solana.com',
};

const main = async (opts: {
  cluster: string;
  gateAuthority: string;
  keypair?: string;
  multisigPubkey?: string;
  vaultIndex: number;
}) => {
  const { cluster, gateAuthority, keypair, multisigPubkey, vaultIndex } = opts;

  console.log(`\n═══════════════════════════════════════════════════════`);
  console.log(`  Setup ACL — cluster: ${cluster}`);
  console.log(`═══════════════════════════════════════════════════════\n`);

  const { rpc } = getRpc(cluster);
  const gateAuth = address(gateAuthority);
  console.log(`Gate Authority: ${gateAuth} (immutable list owner)`);

  const allowListSeed = gateAuth;
  const blockListSeed = TOKEN_ACL_GATE_PROGRAM_PROGRAM_ADDRESS;

  const [allowListPda] = await findListConfigPda({ authority: gateAuth, seed: allowListSeed });
  const [blockListPda] = await findListConfigPda({ authority: gateAuth, seed: blockListSeed });

  console.log(`\nAllow List PDA: ${allowListPda}`);
  console.log(`Block List PDA: ${blockListPda}`);

  console.log(`\nInstructions:\n`);

  const vaultSigner = { address: gateAuth } as any;

  const allowIx = getCreateListInstruction({
    authority: vaultSigner,
    payer: vaultSigner,
    listConfig: allowListPda,
    mode: Mode.Allow,
    seed: allowListSeed,
  });
  printInstruction('Create Allow List', allowIx);

  const blockIx = getCreateListInstruction({
    authority: vaultSigner,
    payer: vaultSigner,
    listConfig: blockListPda,
    mode: Mode.Block,
    seed: blockListSeed,
  });
  printInstruction('Create Block List', blockIx);

  // On devnet, execute via Squads SDK
  if (cluster === 'devnet' && multisigPubkey && keypair) {
    const rpcUrl = CLUSTER_URLS[cluster];

    await executeSquadsProposal({
      rpcUrl,
      keypairPath: keypair,
      multisigPubkey,
      vaultIndex,
      instructions: [allowIx],
      label: 'Create Allow List',
    });

    await executeSquadsProposal({
      rpcUrl,
      keypairPath: keypair,
      multisigPubkey,
      vaultIndex,
      instructions: [blockIx],
      label: 'Create Block List',
    });
  } else if (cluster !== 'devnet') {
    console.log(`\n  ℹ On mainnet, create and execute these proposals via the Squads web UI.`);
  }

  const output = {
    cluster,
    gateAuthority: gateAuth,
    allowListPda,
    blockListPda,
    allowListSeed,
    blockListSeed,
    setupAt: new Date().toISOString(),
  };

  const outDir = path.resolve(process.cwd(), 'deployments');
  if (!fs.existsSync(outDir)) fs.mkdirSync(outDir, { recursive: true });
  const outPath = path.join(outDir, `acl-${cluster}.json`);
  fs.writeFileSync(outPath, JSON.stringify(output, null, 2));

  console.log(`\n\n  Output saved: ${outPath}`);
  console.log(`═══════════════════════════════════════════════════════\n`);
};

const {
  values: {
    cluster,
    'gate-authority': gateAuthority,
    keypair,
    'multisig-pubkey': multisigPubkey,
    'vault-index': vaultIndexStr,
  },
} = parseArgs({
  options: {
    cluster: { type: 'string', default: 'devnet' },
    'gate-authority': { type: 'string' },
    keypair: { type: 'string' },
    'multisig-pubkey': { type: 'string' },
    'vault-index': { type: 'string', default: '0' },
  },
  strict: true,
});

if (!gateAuthority) {
  console.error('Usage: pnpm tsx scripts/setup-acl.ts \\');
  console.error('  --cluster devnet \\');
  console.error('  --gate-authority <squads-vault-pubkey> \\');
  console.error('  --keypair ./deployer.json \\');
  console.error('  --multisig-pubkey <multisig-pubkey> \\');
  console.error('  --vault-index 0');
  process.exit(1);
}

main({
  cluster: cluster!,
  gateAuthority,
  keypair,
  multisigPubkey,
  vaultIndex: Number(vaultIndexStr),
}).catch((e) => {
  console.error(e);
  process.exit(1);
});
