/**
 * upgrade-minter.ts
 *
 * Reproducible upgrade + verification of an already-deployed Minter program.
 *
 * The artifact that gets deployed is the one produced by `solana-verify build` (Docker,
 * image tag driven by `Cargo.toml [workspace.metadata.cli] solana`), NOT the one produced by
 * `anchor build`. Only the Docker build is byte-for-byte reproducible by OtterSec, so deploying
 * anything else would make the program unverifiable.
 *
 * Flow:
 *   1. Preflight  — clean tree, commit pushed to origin, on-chain upgrade authority matches keypair
 *   2. Build      — anchor build (IDL) → generate-clients drift check → solana-verify build (.so)
 *   3. Guards     — no-op if the on-chain hash already matches; abort if the binary outgrew the account
 *   4. Deploy     — solana program deploy, then assert the on-chain hash equals the local one
 *   5. Verify     — solana-verify verify-from-repo --remote, then poll the OtterSec status endpoint
 *   6. Record     — update deployments/minter-<cluster>.json
 *
 * Usage:
 *   pnpm tsx scripts/upgrade-minter.ts \
 *     --cluster mainnet-beta \
 *     --keypair ./deployer.json \
 *     --rpc-url https://my-private-rpc.example.com
 */
import { parseArgs } from 'node:util';
import { execFileSync } from 'node:child_process';
import fs from 'fs';
import path from 'path';
import { address } from '@solana/kit';
import { MINTER_PROGRAM_ADDRESS } from '../clients/ts/minter/src/generated/programs/index.js';
import { loadKeypair } from './lib/loadKeypair.js';
import { getRpc } from './lib/getRpc.js';

const CLUSTER_URLS: Record<string, string> = {
  devnet: 'https://api.devnet.solana.com',
  'mainnet-beta': 'https://api.mainnet-beta.solana.com',
};

const DEFAULT_REPO_URL = 'https://github.com/spiko-tech/solana-contracts';
const OSEC_STATUS_URL = 'https://verify.osec.io/status';
const SO_PATH = 'target/deploy/minter.so';

/** BPF Loader 2 — the non-upgradeable loader. A program owned by it can never be upgraded. */
const NON_UPGRADEABLE_LOADER = 'BPFLoader2111111111111111111111111111111111';

const run = (cmd: string, args: string[]): string => execFileSync(cmd, args, { encoding: 'utf-8' }).trim();

const runInherit = (cmd: string, args: string[]) => execFileSync(cmd, args, { stdio: 'inherit' });

const banner = (title: string) => {
  console.log(`\n═══════════════════════════════════════════════════════`);
  console.log(`  ${title}`);
  console.log(`═══════════════════════════════════════════════════════\n`);
};

const step = (title: string) => console.log(`\n── ${title} ──`);

/** Reads `[workspace.metadata.cli] solana` — the pin that selects the verifiable-build image. */
const readAgavePin = (): string => {
  const cargoToml = fs.readFileSync(path.resolve(process.cwd(), 'Cargo.toml'), 'utf-8');
  const section = cargoToml.split(/^\[workspace\.metadata\.cli\]$/m)[1];
  const match = section?.split(/^\[/m)[0].match(/^\s*solana\s*=\s*"([^"]+)"/m);
  if (!match) throw new Error('Could not read [workspace.metadata.cli] solana from Cargo.toml');
  return match[1];
};

const preflight = async (cluster: string, keypairPath: string, rpcUrl: string) => {
  step('Preflight');

  if (run('git', ['status', '--porcelain'])) {
    throw new Error('Working tree is dirty. Commit or stash before upgrading — the deployed binary must match a pushed commit.');
  }

  const commit = run('git', ['rev-parse', 'HEAD']);
  const remoteBranches = run('git', ['branch', '-r', '--contains', commit]);
  if (!remoteBranches) {
    throw new Error(
      `Commit ${commit} is not on any remote branch.\n` +
        `  OtterSec clones the repo from GitHub, so an unpushed commit cannot be verified.\n` +
        `  Push to origin first.`,
    );
  }
  console.log(`  Commit:              ${commit}`);
  console.log(
    `  Remote branches:     ${remoteBranches
      .split('\n')
      .map((b) => b.trim())
      .join(', ')}`,
  );

  const deploymentPath = path.resolve(process.cwd(), 'deployments', `minter-${cluster}.json`);
  if (!fs.existsSync(deploymentPath)) {
    throw new Error(`${deploymentPath} not found. Run setup-minter.ts first — this script only upgrades an existing deployment.`);
  }
  const deployment = JSON.parse(fs.readFileSync(deploymentPath, 'utf-8'));
  if (deployment.program !== MINTER_PROGRAM_ADDRESS) {
    throw new Error(`Program ID mismatch!\n  deployments file: ${deployment.program}\n  declare_id:       ${MINTER_PROGRAM_ADDRESS}`);
  }

  const signer = await loadKeypair(keypairPath);
  const { rpc } = getRpc(cluster);

  const { value: programAccount } = await rpc.getAccountInfo(address(MINTER_PROGRAM_ADDRESS), { encoding: 'base64' }).send();
  if (!programAccount) throw new Error(`Program ${MINTER_PROGRAM_ADDRESS} not found on ${cluster}. Nothing to upgrade.`);
  if (programAccount.owner === NON_UPGRADEABLE_LOADER) {
    throw new Error(`Program ${MINTER_PROGRAM_ADDRESS} is owned by the non-upgradeable loader — it cannot be upgraded.`);
  }

  const showOutput = run('solana', ['program', 'show', MINTER_PROGRAM_ADDRESS, '--url', rpcUrl]);
  const field = (label: string) =>
    showOutput
      .split('\n')
      .find((l) => l.startsWith(`${label}:`))
      ?.split(':')
      .slice(1)
      .join(':')
      .trim();

  const programDataAddress = field('ProgramData Address');
  const onChainAuthority = field('Authority');
  const onChainDataLength = Number(field('Data Length')?.split(' ')[0]);
  if (!Number.isFinite(onChainDataLength)) {
    throw new Error(`Could not parse the on-chain data length from 'solana program show':\n${showOutput}`);
  }

  if (onChainAuthority !== signer.address) {
    throw new Error(
      `Upgrade authority mismatch!\n` +
        `  On-chain authority: ${onChainAuthority}\n` +
        `  Provided keypair:   ${signer.address}\n` +
        `  Use the keypair that currently holds the upgrade authority.`,
    );
  }

  console.log(`  Program:             ${MINTER_PROGRAM_ADDRESS}`);
  console.log(`  ProgramData:         ${programDataAddress}`);
  console.log(`  Upgrade authority:   ${onChainAuthority} (matches keypair)`);
  console.log(`  On-chain size:       ${onChainDataLength} bytes`);

  return { commit, deployment, deploymentPath, signer, programDataAddress, onChainDataLength };
};

const build = () => {
  step('Building IDL + TypeScript clients (anchor)');
  runInherit('anchor', ['build', '-p', 'minter', '--ignore-keys']);
  runInherit('pnpm', ['generate-clients']);
  const clientDrift = run('git', ['status', '--porcelain', '--', 'clients/ts']);
  if (clientDrift) {
    throw new Error(
      `clients/ts is out of date with the program IDL:\n${clientDrift}\n` +
        `  Run 'pnpm generate-clients' and commit the result before upgrading.`,
    );
  }
  console.log(`  ✓ Generated client is up to date`);

  const agavePin = readAgavePin();
  step(`Reproducible build (solanafoundation/solana-verifiable-build:${agavePin})`);
  console.log(`  This overwrites ${SO_PATH} with the Docker-built artifact — that is the one we deploy.`);
  runInherit('solana-verify', ['build', '--library-name', 'minter']);

  if (!fs.existsSync(SO_PATH)) throw new Error(`${SO_PATH} not found after solana-verify build`);
  const executableHash = run('solana-verify', ['get-executable-hash', SO_PATH]);
  const size = fs.statSync(SO_PATH).size;
  console.log(`  ✓ Executable hash:   ${executableHash}`);
  console.log(`  ✓ Size:              ${size} bytes`);

  return { executableHash, size, agavePin };
};

const getProgramHash = (rpcUrl: string): string => run('solana-verify', ['get-program-hash', '-u', rpcUrl, MINTER_PROGRAM_ADDRESS]);

const deploy = (rpcUrl: string, keypairPath: string, signerAddress: string) => {
  step('Deploying program');
  try {
    runInherit('solana', [
      'program',
      'deploy',
      SO_PATH,
      '--program-id',
      MINTER_PROGRAM_ADDRESS,
      '-u',
      rpcUrl,
      '-k',
      keypairPath,
      '--use-rpc',
    ]);
  } catch (e) {
    console.error(
      `\n✗ Deploy failed. Funds may be parked in an intermediate buffer.\n` +
        `  Inspect:  solana program show --buffers --buffer-authority ${signerAddress} -u ${rpcUrl}\n` +
        `  Resume:   solana program deploy --buffer <BUFFER> --program-id ${MINTER_PROGRAM_ADDRESS} -u ${rpcUrl} -k ${keypairPath}\n` +
        `  Reclaim:  solana program close <BUFFER> -u ${rpcUrl} -k ${keypairPath}\n`,
    );
    throw e;
  }
  console.log(`  ✓ Program deployed`);
};

const fetchVerifyStatus = async (): Promise<Record<string, unknown>> => {
  const res = await fetch(`${OSEC_STATUS_URL}/${MINTER_PROGRAM_ADDRESS}`);
  if (!res.ok) throw new Error(`OtterSec status endpoint returned ${res.status}`);
  return (await res.json()) as Record<string, unknown>;
};

const verify = async (repoUrl: string, commit: string, rpcUrl: string, keypairPath: string, uploader: string) => {
  // Two steps since solana-verify 0.5.x: the `--remote` flag is deprecated. First the upgrade
  // authority writes the verify PDA on chain, then the remote build worker is queued against it.
  step('Uploading verification PDA');
  console.log(`  Repo:   ${repoUrl}`);
  console.log(`  Commit: ${commit}`);
  runInherit('solana-verify', [
    'verify-from-repo',
    repoUrl,
    '--program-id',
    MINTER_PROGRAM_ADDRESS,
    '--library-name',
    'minter',
    '--commit-hash',
    commit,
    '-u',
    rpcUrl,
    '-k',
    keypairPath,
    '-y',
  ]);

  step('Queueing the remote verification job');
  runInherit('solana-verify', ['remote', 'submit-job', '--program-id', MINTER_PROGRAM_ADDRESS, '--uploader', uploader, '-u', rpcUrl]);

  step('Polling verification status');
  for (let attempt = 1; attempt <= 40; attempt++) {
    const status = await fetchVerifyStatus();
    if (status.is_verified) {
      console.log(`  ✓ Verified — repo ${status.repo_url} @ ${status.commit}`);
      return true;
    }
    console.log(`  [${attempt}/40] ${status.message ?? 'pending'}`);
    await new Promise((r) => setTimeout(r, 15_000));
  }
  console.warn(`  ⚠ Still not verified after ~10 minutes. Check ${OSEC_STATUS_URL}/${MINTER_PROGRAM_ADDRESS} later.`);
  return false;
};

const saveDeployment = (
  deploymentPath: string,
  deployment: Record<string, unknown>,
  fields: {
    programDataAddress?: string;
    commit: string;
    executableHash: string;
    agavePin: string;
    verified: boolean;
  },
) => {
  const output = {
    ...deployment,
    programDataAddress: fields.programDataAddress,
    deployedCommit: fields.commit,
    executableHash: fields.executableHash,
    verifiableBuildImage: `solanafoundation/solana-verifiable-build:${fields.agavePin}`,
    verified: fields.verified,
    lastUpgradeAt: new Date().toISOString(),
  };
  fs.writeFileSync(deploymentPath, `${JSON.stringify(output, null, 2)}\n`);
  return deploymentPath;
};

const main = async ({
  cluster,
  keypair,
  repoUrl,
  rpcUrl,
  skipVerify,
  dryRun,
}: {
  cluster: string;
  keypair: string;
  repoUrl: string;
  rpcUrl: string;
  skipVerify: boolean;
  dryRun: boolean;
}) => {
  banner(`Upgrade Minter — cluster: ${cluster}${dryRun ? ' (DRY RUN)' : ''}`);
  console.log(`RPC: ${rpcUrl}`);

  const { commit, deployment, deploymentPath, signer, programDataAddress, onChainDataLength } = await preflight(cluster, keypair, rpcUrl);
  const { executableHash, size, agavePin } = build();

  step('Comparing against on-chain program');
  const onChainHash = getProgramHash(rpcUrl);
  console.log(`  On-chain hash:  ${onChainHash}`);
  console.log(`  Local hash:     ${executableHash}`);

  let deployed = false;
  if (onChainHash === executableHash) {
    console.log(`  ✓ Already up to date — skipping deploy.`);
  } else if (dryRun) {
    console.log(`  → Deploy needed. Stopping here (--dry-run).`);
  } else {
    if (size > onChainDataLength) {
      throw new Error(
        `Binary grew beyond the on-chain account (${size} > ${onChainDataLength} bytes).\n` +
          `  Extend it first:\n` +
          `    solana program extend ${MINTER_PROGRAM_ADDRESS} ${size - onChainDataLength} -u ${rpcUrl} -k ${keypair}\n` +
          `  Then re-run this script.`,
      );
    }
    deploy(rpcUrl, keypair, signer.address);
    deployed = true;

    step('Verifying deployed bytes');
    const postHash = getProgramHash(rpcUrl);
    if (postHash !== executableHash) {
      throw new Error(`Post-deploy hash mismatch!\n  On-chain: ${postHash}\n  Expected: ${executableHash}`);
    }
    console.log(`  ✓ On-chain hash matches the reproducible build`);
  }

  if (dryRun) {
    console.log(`\nDry run complete — no chain state was modified.\n`);
    return;
  }

  const verified = skipVerify ? false : await verify(repoUrl, commit, rpcUrl, keypair, signer.address);

  const outPath = saveDeployment(deploymentPath, deployment, { programDataAddress, commit, executableHash, agavePin, verified });

  banner('Upgrade complete');
  console.log(`  Deployed:  ${deployed ? 'yes' : 'no (already up to date)'}`);
  console.log(`  Verified:  ${verified ? 'yes' : skipVerify ? 'skipped' : 'pending — check the status endpoint'}`);
  console.log(`  Record:    ${outPath}`);
  console.log(`  Explorer:  https://solscan.io/account/${MINTER_PROGRAM_ADDRESS}\n`);
};

const {
  values: { cluster, keypair, 'repo-url': repoUrl, 'rpc-url': rpcUrlArg, 'skip-verify': skipVerify, 'dry-run': dryRun },
} = parseArgs({
  options: {
    cluster: { type: 'string', default: 'mainnet-beta' },
    keypair: { type: 'string' },
    'repo-url': { type: 'string', default: DEFAULT_REPO_URL },
    'rpc-url': { type: 'string' },
    'skip-verify': { type: 'boolean', default: false },
    'dry-run': { type: 'boolean', default: false },
  },
  strict: true,
});

if (!keypair) {
  console.error('Usage: pnpm tsx scripts/upgrade-minter.ts \\');
  console.error('  --cluster mainnet-beta \\');
  console.error('  --keypair ./deployer.json \\');
  console.error('  [--rpc-url <url>] [--repo-url <url>] [--skip-verify] [--dry-run]');
  process.exit(1);
}

const rpcUrl = rpcUrlArg ?? CLUSTER_URLS[cluster];
if (!rpcUrl) {
  console.error(`Unknown cluster: ${cluster} (expected 'devnet' or 'mainnet-beta', or pass --rpc-url)`);
  process.exit(1);
}

main({ cluster, keypair, repoUrl: repoUrl!, rpcUrl, skipVerify: skipVerify!, dryRun: dryRun! }).catch((e) => {
  console.error(e instanceof Error ? `\n✗ ${e.message}\n` : e);
  process.exit(1);
});
