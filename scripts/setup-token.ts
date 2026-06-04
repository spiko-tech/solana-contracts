/**
 * setup-token.ts
 *
 * Creates a new Token-2022 mint with all Spiko extensions, configures Token ACL,
 * and transfers all authorities to their final owners.
 *
 * The keypair is the temporary authority for everything. After execution:
 * - Mint authority → Minter Config PDA
 * - Freeze authority → Token ACL MintConfig PDA (via createConfig)
 * - Metadata update authority → --metadata-update-authority
 * - Token ACL config authority → --token-acl-authority
 * - Permanent delegate, metadata pointer, pause, mint close → set at init (immutable)
 *
 * Squads proposals are printed (not executed) for:
 * - Minter setDailyLimit (needs Minter admin)
 *
 * Usage:
 *   pnpm tsx scripts/setup-token.ts \
 *     --cluster devnet \
 *     --keypair ./deployer.json \
 *     --symbol EUTBL \
 *     --name "EU T-Bill Token" \
 *     --uri "https://spiko.finance/eutbl" \
 *     --decimals 6 \
 *     --minter-daily-limit 1000000 \
 *     --permanent-delegate <pubkey> \
 *     --metadata-pointer-authority <pubkey> \
 *     --metadata-update-authority <pubkey> \
 *     --pause-authority <pubkey> \
 *     --mint-close-authority <pubkey> \
 *     --token-acl-authority <pubkey>
 */

import { parseArgs } from 'node:util';
import fs from 'fs';
import path from 'path';
import {
  type Address,
  type KeyPairSigner,
  type Rpc,
  type SolanaRpcApi,
  type RpcSubscriptions,
  type SolanaRpcSubscriptionsApi,
  address,
  generateKeyPairSigner,
  AccountRole,
} from '@solana/kit';
import { getCreateAccountInstruction, getTransferSolInstruction } from '@solana-program/system';
import {
  getInitializeMint2Instruction,
  getInitializeDefaultAccountStateInstruction,
  getInitializePermanentDelegateInstruction,
  getInitializeMetadataPointerInstruction,
  getInitializePausableConfigInstruction,
  getInitializeMintCloseAuthorityInstruction,
  getInitializeTokenMetadataInstruction,
  getUpdateTokenMetadataUpdateAuthorityInstruction,
  getSetAuthorityInstruction,
  getMintSize,
  extension,
  AccountState,
  AuthorityType,
  TOKEN_2022_PROGRAM_ADDRESS,
} from '@solana-program/token-2022';
import {
  getCreateConfigInstructionAsync,
  getTogglePermissionlessInstructionsInstruction,
  getSetAuthorityInstruction as getTokenAclSetAuthorityInstruction,
  findMintConfigPda as findTokenAclMintConfigPda,
  findThawExtraMetasAccountPda,
  setTokenAclMetadata,
} from '@solana/token-acl-sdk';
import { getSetupExtraMetasInstruction, TOKEN_ACL_GATE_PROGRAM_PROGRAM_ADDRESS } from '@solana/token-acl-gate-sdk';
import { printInstruction } from './lib/printInstruction.js';
import { executeSquadsProposal } from './lib/executeSquadsProposal.js';
import { loadKeypair } from './lib/loadKeypair.js';
import { getSetDailyLimitInstructionAsync } from '../clients/ts/minter/src/generated/instructions/setDailyLimit.js';
import { findMinterConfigPda } from '../clients/ts/minter/src/generated/pdas/index.js';
import { getRpc } from './lib/getRpc.js';
import { sendTx } from './lib/sendTx.js';

const getMinimumRent = async ({ rpc, space }: { rpc: Rpc<SolanaRpcApi>; space: number }): Promise<bigint> => {
  const result = await rpc.getMinimumBalanceForRentExemption(BigInt(space)).send();
  return result as unknown as bigint;
};

const loadAclConfig = (
  cluster: string,
): {
  allowListPda: Address;
  blockListPda: Address;
  gateAuthority: Address;
} => {
  const aclPath = path.resolve(process.cwd(), 'deployments', `acl-${cluster}.json`);
  if (!fs.existsSync(aclPath)) throw new Error(`Missing ${aclPath} — run setup-acl.ts first.`);
  const raw = JSON.parse(fs.readFileSync(aclPath, 'utf-8'));
  console.log(`ACL config loaded: ${aclPath}`);
  console.log(`  Allow List: ${raw.allowListPda}`);
  console.log(`  Block List: ${raw.blockListPda}`);
  console.log(`  Gate Authority (ABL): ${raw.gateAuthority}\n`);
  return {
    allowListPda: address(raw.allowListPda),
    blockListPda: address(raw.blockListPda),
    gateAuthority: address(raw.gateAuthority),
  };
};

const buildPreMintExtensions = ({
  mint,
  permanentDelegate,
  metadataPointerAuthority,
  pauseAuthority,
  mintCloseAuthority,
}: {
  mint: Address;
  permanentDelegate: Address;
  metadataPointerAuthority: Address;
  pauseAuthority: Address;
  mintCloseAuthority: Address;
}) => {
  return [
    extension('DefaultAccountState', { state: AccountState.Frozen }),
    extension('PermanentDelegate', { delegate: permanentDelegate }),
    extension('MetadataPointer', { authority: metadataPointerAuthority, metadataAddress: mint }),
    extension('PausableConfig', { authority: pauseAuthority, paused: false }),
    extension('MintCloseAuthority', { closeAuthority: mintCloseAuthority }),
  ];
};

const buildFullExtensions = ({
  mint,
  permanentDelegate,
  metadataPointerAuthority,
  pauseAuthority,
  mintCloseAuthority,
  name,
  symbol,
  uri,
}: {
  mint: Address;
  permanentDelegate: Address;
  metadataPointerAuthority: Address;
  pauseAuthority: Address;
  mintCloseAuthority: Address;
  name: string;
  symbol: string;
  uri: string;
}) => {
  return [
    ...buildPreMintExtensions({ mint, permanentDelegate, metadataPointerAuthority, pauseAuthority, mintCloseAuthority }),
    extension('TokenMetadata', {
      updateAuthority: address('11111111111111111111111111111111'), // placeholder for size calc
      mint,
      name,
      symbol,
      uri,
      additionalMetadata: new Map([['token_acl', String(TOKEN_ACL_GATE_PROGRAM_PROGRAM_ADDRESS)]]),
    }),
  ];
};

const createMintWithExtensions = async ({
  rpc,
  rpcSub,
  admin,
  mintKp,
  symbol,
  decimals,
  permanentDelegate,
  metadataPointerAuthority,
  pauseAuthority,
  mintCloseAuthority,
}: {
  rpc: Rpc<SolanaRpcApi>;
  rpcSub: RpcSubscriptions<SolanaRpcSubscriptionsApi>;
  admin: KeyPairSigner;
  mintKp: KeyPairSigner;
  symbol: string;
  decimals: number;
  permanentDelegate: Address;
  metadataPointerAuthority: Address;
  pauseAuthority: Address;
  mintCloseAuthority: Address;
}) => {
  const mint = mintKp.address;

  // Only include pre-mint extensions for initial space calculation
  // TokenMetadata will be added via InitializeTokenMetadata which handles realloc
  const preMintExtensions = buildPreMintExtensions({
    mint,
    permanentDelegate,
    metadataPointerAuthority,
    pauseAuthority,
    mintCloseAuthority,
  });
  const space = getMintSize(preMintExtensions);
  const mintRent = await getMinimumRent({ rpc, space });

  console.log(`\n── TX 1: Create mint + init extensions ──`);
  await sendTx(
    rpc,
    rpcSub,
    admin,
    [
      getCreateAccountInstruction({
        payer: admin,
        newAccount: mintKp,
        lamports: mintRent,
        space,
        programAddress: TOKEN_2022_PROGRAM_ADDRESS,
      }),
      getInitializeDefaultAccountStateInstruction({ mint, state: AccountState.Frozen }),
      getInitializePermanentDelegateInstruction({ mint, delegate: permanentDelegate }),
      getInitializeMetadataPointerInstruction({ mint, authority: metadataPointerAuthority, metadataAddress: mint }),
      getInitializePausableConfigInstruction({ mint, authority: pauseAuthority }),
      getInitializeMintCloseAuthorityInstruction({ mint, closeAuthority: mintCloseAuthority }),
      getInitializeMint2Instruction({ mint, decimals, mintAuthority: admin.address, freezeAuthority: admin.address }),
    ],
    `Create mint ${symbol}`,
  );
};

const initializeMetadata = async ({
  rpc,
  rpcSub,
  admin,
  mint,
  permanentDelegate,
  metadataPointerAuthority,
  pauseAuthority,
  mintCloseAuthority,
  name,
  symbol,
  uri,
}: {
  rpc: Rpc<SolanaRpcApi>;
  rpcSub: RpcSubscriptions<SolanaRpcSubscriptionsApi>;
  admin: KeyPairSigner;
  mint: Address;
  permanentDelegate: Address;
  metadataPointerAuthority: Address;
  pauseAuthority: Address;
  mintCloseAuthority: Address;
  name: string;
  symbol: string;
  uri: string;
}) => {
  const preMintExtensions = buildPreMintExtensions({
    mint,
    permanentDelegate,
    metadataPointerAuthority,
    pauseAuthority,
    mintCloseAuthority,
  });
  const withMetadataExtensions = [
    ...preMintExtensions,
    extension('TokenMetadata', { updateAuthority: admin.address, mint, name, symbol, uri, additionalMetadata: new Map<string, string>() }),
  ];

  const currentRent = await getMinimumRent({ rpc, space: getMintSize(preMintExtensions) });
  const newRent = await getMinimumRent({ rpc, space: getMintSize(withMetadataExtensions) });
  const extraLamports = newRent - currentRent;

  console.log(`\n── TX 2: Initialize metadata ──`);
  await sendTx(
    rpc,
    rpcSub,
    admin,
    [
      getTransferSolInstruction({ source: admin, destination: mint, amount: extraLamports }),
      getInitializeTokenMetadataInstruction({
        metadata: mint,
        updateAuthority: admin.address,
        mint,
        mintAuthority: admin,
        name,
        symbol,
        uri,
      }),
    ],
    `Init metadata ${symbol}`,
  );
};

const addTokenAclField = async ({
  rpc,
  rpcSub,
  admin,
  mint,
  permanentDelegate,
  metadataPointerAuthority,
  pauseAuthority,
  mintCloseAuthority,
  name,
  symbol,
  uri,
}: {
  rpc: Rpc<SolanaRpcApi>;
  rpcSub: RpcSubscriptions<SolanaRpcSubscriptionsApi>;
  admin: KeyPairSigner;
  mint: Address;
  permanentDelegate: Address;
  metadataPointerAuthority: Address;
  pauseAuthority: Address;
  mintCloseAuthority: Address;
  name: string;
  symbol: string;
  uri: string;
}) => {
  const baseExtensions = [
    ...buildPreMintExtensions({ mint, permanentDelegate, metadataPointerAuthority, pauseAuthority, mintCloseAuthority }),
    extension('TokenMetadata', { updateAuthority: admin.address, mint, name, symbol, uri, additionalMetadata: new Map<string, string>() }),
  ];

  const fullExtensions = buildFullExtensions({
    mint,
    permanentDelegate,
    metadataPointerAuthority,
    pauseAuthority,
    mintCloseAuthority,
    name,
    symbol,
    uri,
  });

  const baseRent = await getMinimumRent({ rpc, space: getMintSize(baseExtensions) });
  const fullRent = await getMinimumRent({ rpc, space: getMintSize(fullExtensions) });
  const extraLamports = fullRent - baseRent;

  console.log(`\n── TX 3: Add token_acl metadata field ──`);
  await sendTx(
    rpc,
    rpcSub,
    admin,
    [
      getTransferSolInstruction({ source: admin, destination: mint, amount: extraLamports }),
      setTokenAclMetadata(admin, mint, TOKEN_ACL_GATE_PROGRAM_PROGRAM_ADDRESS),
    ],
    `Add token_acl field`,
  );
};

const configureTokenAcl = async ({
  rpc,
  rpcSub,
  admin,
  mint,
  tokenAclMintConfigPda,
}: {
  rpc: Rpc<SolanaRpcApi>;
  rpcSub: RpcSubscriptions<SolanaRpcSubscriptionsApi>;
  admin: KeyPairSigner;
  mint: Address;
  tokenAclMintConfigPda: Address;
}) => {
  console.log(`\n── TX 4: Create Token ACL config ──`);
  const createConfigIx = await getCreateConfigInstructionAsync({
    payer: admin.address,
    authority: admin,
    mint,
    gatingProgram: TOKEN_ACL_GATE_PROGRAM_PROGRAM_ADDRESS,
  });
  await sendTx(rpc, rpcSub, admin, [createConfigIx], `Create Token ACL config`);

  console.log(`\n── TX 5: Enable permissionless thaw ──`);
  const toggleIx = getTogglePermissionlessInstructionsInstruction({
    authority: admin,
    mintConfig: tokenAclMintConfigPda,
    thawEnabled: true,
    freezeEnabled: false,
  });
  await sendTx(rpc, rpcSub, admin, [toggleIx], `Enable permissionless thaw`);
};

const transferAllAuthorities = async ({
  rpc,
  rpcSub,
  admin,
  mint,
  minterConfigPda,
  metadataUpdateAuthority,
  tokenAclMintConfigPda,
  thawExtraMetasPda,
  allowListPda,
  blockListPda,
  tokenAclAuthority,
}: {
  rpc: Rpc<SolanaRpcApi>;
  rpcSub: RpcSubscriptions<SolanaRpcSubscriptionsApi>;
  admin: KeyPairSigner;
  mint: Address;
  minterConfigPda: Address;
  metadataUpdateAuthority: Address;
  tokenAclMintConfigPda: Address;
  thawExtraMetasPda: Address;
  allowListPda: Address;
  blockListPda: Address;
  tokenAclAuthority: Address;
}) => {
  console.log(`\n── TX 6: Transfer authorities ──`);

  await sendTx(
    rpc,
    rpcSub,
    admin,
    [getSetAuthorityInstruction({ owned: mint, owner: admin, authorityType: AuthorityType.MintTokens, newAuthority: minterConfigPda })],
    `Transfer mint authority → Minter Config PDA`,
  );

  await sendTx(
    rpc,
    rpcSub,
    admin,
    [
      getUpdateTokenMetadataUpdateAuthorityInstruction({
        metadata: mint,
        updateAuthority: admin,
        newUpdateAuthority: metadataUpdateAuthority,
      }),
    ],
    `Transfer metadata update authority → ${metadataUpdateAuthority}`,
  );

  const setupMetasIx = getSetupExtraMetasInstruction({
    authority: admin,
    payer: admin,
    tokenAclMintConfig: tokenAclMintConfigPda,
    mint,
    extraMetas: thawExtraMetasPda,
    lists: [allowListPda, blockListPda],
  });
  await sendTx(rpc, rpcSub, admin, [setupMetasIx], `ABL Gate: Setup Extra Metas`);

  const tokenAclSetAuthIx = getTokenAclSetAuthorityInstruction({
    authority: admin,
    mintConfig: tokenAclMintConfigPda,
    newAuthority: tokenAclAuthority,
  });
  await sendTx(rpc, rpcSub, admin, [tokenAclSetAuthIx], `Transfer Token ACL authority → ${tokenAclAuthority}`);
};

const saveDeployment = ({
  cluster,
  mint,
  keypair,
  symbol,
  name,
  uri,
  decimals,
  minterDailyLimit,
  permanentDelegate,
  metadataPointerAuthority,
  metadataUpdateAuthority,
  pauseAuthority,
  mintCloseAuthority,
  tokenAclAuthority,
  minterConfigPda,
  tokenAclMintConfigPda,
  allowListPda,
  blockListPda,
  gateAuthority,
}: {
  cluster: string;
  mint: Address;
  keypair: string;
  symbol: string;
  name: string;
  uri: string;
  decimals: number;
  minterDailyLimit: number;
  permanentDelegate: string;
  metadataPointerAuthority: string;
  metadataUpdateAuthority: string;
  pauseAuthority: string;
  mintCloseAuthority: string;
  tokenAclAuthority: string;
  minterConfigPda: Address;
  tokenAclMintConfigPda: Address;
  allowListPda: Address;
  blockListPda: Address;
  gateAuthority: Address;
}): string => {
  const outDir = path.resolve(process.cwd(), 'deployments');
  if (!fs.existsSync(outDir)) fs.mkdirSync(outDir, { recursive: true });

  const output = {
    cluster,
    mint,
    symbol,
    name,
    uri,
    decimals,
    authorities: {
      mintAuthority: minterConfigPda,
      freezeAuthority: tokenAclMintConfigPda,
      permanentDelegate,
      metadataPointerAuthority,
      metadataUpdateAuthority,
      pauseAuthority,
      mintCloseAuthority,
      tokenAclAuthority,
    },
    tokenAclMintConfig: tokenAclMintConfigPda,
    minterDailyLimit,
    allowListPda,
    blockListPda,
    gateAuthority,
    setupAt: new Date().toISOString(),
  };

  const outPath = path.join(outDir, `${symbol}-${cluster}.json`);
  fs.writeFileSync(outPath, JSON.stringify(output, null, 2));
  return outPath;
};

const printSetDailyLimitProposal = async ({
  rpc,
  mint,
  cluster,
  decimals,
  minterDailyLimit,
  keypair,
  multisigPubkey,
  vaultIndex,
}: {
  rpc: Rpc<SolanaRpcApi>;
  mint: Address;
  cluster: string;
  decimals: number;
  minterDailyLimit: number;
  keypair: string;
  multisigPubkey?: string;
  vaultIndex: number;
}) => {
  const minterDeployPath = path.resolve(process.cwd(), 'deployments', `minter-${cluster}.json`);
  if (!fs.existsSync(minterDeployPath)) {
    console.log(`  ⚠ Cannot generate setDailyLimit instruction — ${minterDeployPath} not found.`);
    console.log(`    Run setup-minter.ts first, then manually set daily limit via Squads.`);
    return;
  }

  const minterConfig = JSON.parse(fs.readFileSync(minterDeployPath, 'utf-8'));
  const minterAdmin = address(minterConfig.minterAdmin);

  const setDailyLimitIx = await getSetDailyLimitInstructionAsync({
    admin: { address: minterAdmin } as any,
    payer: { address: minterAdmin } as any,
    mint,
    limit: BigInt(minterDailyLimit) * BigInt(10 ** decimals),
  });

  const fixedIx = { ...setDailyLimitIx, accounts: [...(setDailyLimitIx as any).accounts] };
  fixedIx.accounts[0] = { ...fixedIx.accounts[0], role: AccountRole.READONLY_SIGNER };
  fixedIx.accounts[4] = { ...fixedIx.accounts[4], role: AccountRole.WRITABLE_SIGNER };
  printInstruction('Minter: Set Daily Limit', fixedIx);

  // On devnet, execute via Squads SDK
  if (cluster === 'devnet' && multisigPubkey) {
    const rpcUrl = 'https://api.devnet.solana.com';
    await executeSquadsProposal({
      rpcUrl,
      keypairPath: keypair,
      multisigPubkey,
      vaultIndex,
      instructions: [fixedIx],
      label: 'Set Daily Limit',
    });
  } else if (cluster !== 'devnet') {
    console.log(`\n  ℹ On mainnet, create and execute this proposal via the Squads web UI.`);
  }
};

const main = async (args: {
  cluster: string;
  keypair: string;
  symbol: string;
  name: string;
  uri: string;
  decimals: number;
  minterDailyLimit: number;
  permanentDelegate: string;
  metadataPointerAuthority: string;
  metadataUpdateAuthority: string;
  pauseAuthority: string;
  mintCloseAuthority: string;
  tokenAclAuthority: string;
  multisigPubkey?: string;
  vaultIndex: number;
}) => {
  console.log(`\n═══════════════════════════════════════════════════════`);
  console.log(`  Setup Token: ${args.symbol} — cluster: ${args.cluster}`);
  console.log(`═══════════════════════════════════════════════════════\n`);

  const aclConfig = loadAclConfig(args.cluster);
  const admin = await loadKeypair(args.keypair);
  const { rpc, rpcSub } = getRpc(args.cluster);

  console.log(`Keypair (temp authority): ${admin.address}`);

  const mintKp = await generateKeyPairSigner();
  const mint = mintKp.address;
  console.log(`Mint address: ${mint}`);

  const [minterConfigPda] = await findMinterConfigPda();
  const [tokenAclMintConfigPda] = await findTokenAclMintConfigPda({ mint });
  const [thawExtraMetasPda] = await findThawExtraMetasAccountPda({ mint }, { programAddress: TOKEN_ACL_GATE_PROGRAM_PROGRAM_ADDRESS });

  console.log(`Minter Config PDA: ${minterConfigPda}`);
  console.log(`Token ACL MintConfig PDA: ${tokenAclMintConfigPda}`);

  const cluster = args.cluster;
  const keypair = args.keypair;
  const minterDailyLimit = args.minterDailyLimit;
  const symbol = args.symbol;
  const name = args.name;
  const uri = args.uri;
  const decimals = args.decimals;
  const permanentDelegate = address(args.permanentDelegate);
  const metadataPointerAuthority = address(args.metadataPointerAuthority);
  const metadataUpdateAuthority = address(args.metadataUpdateAuthority);
  const pauseAuthority = address(args.pauseAuthority);
  const mintCloseAuthority = address(args.mintCloseAuthority);
  const tokenAclAuthority = address(args.tokenAclAuthority);

  console.log(`\nAuthorities:`);
  console.log(`  Permanent Delegate:         ${permanentDelegate} (immutable)`);
  console.log(`  Metadata Pointer Authority: ${metadataPointerAuthority}`);
  console.log(`  Metadata Update Authority:  ${metadataUpdateAuthority}`);
  console.log(`  Pause Authority:            ${pauseAuthority}`);
  console.log(`  Mint Close Authority:       ${mintCloseAuthority}`);
  console.log(`  Token ACL Authority:        ${tokenAclAuthority}`);
  console.log(`  Mint Authority (final):     ${minterConfigPda}`);
  console.log(`  Freeze Authority (final):   ${tokenAclMintConfigPda}`);

  await createMintWithExtensions({
    rpc,
    rpcSub,
    admin,
    mintKp,
    symbol: args.symbol,
    decimals: args.decimals,
    permanentDelegate,
    metadataPointerAuthority,
    pauseAuthority,
    mintCloseAuthority,
  });

  await initializeMetadata({
    rpc,
    rpcSub,
    admin,
    mint,
    permanentDelegate,
    metadataPointerAuthority,
    pauseAuthority,
    mintCloseAuthority,
    name,
    symbol,
    uri,
  });

  await addTokenAclField({
    rpc,
    rpcSub,
    admin,
    mint,
    permanentDelegate,
    metadataPointerAuthority,
    pauseAuthority,
    mintCloseAuthority,
    name,
    symbol,
    uri,
  });

  await configureTokenAcl({ rpc, rpcSub, admin, mint, tokenAclMintConfigPda });

  await transferAllAuthorities({
    rpc,
    rpcSub,
    admin,
    mint,
    minterConfigPda,
    metadataUpdateAuthority,
    tokenAclMintConfigPda,
    thawExtraMetasPda,
    allowListPda: aclConfig.allowListPda,
    blockListPda: aclConfig.blockListPda,
    tokenAclAuthority,
  });

  const outPath = saveDeployment({
    cluster,
    mint,
    keypair,
    symbol,
    name,
    uri,
    decimals,
    minterDailyLimit,
    permanentDelegate,
    metadataPointerAuthority,
    metadataUpdateAuthority,
    pauseAuthority,
    mintCloseAuthority,
    tokenAclAuthority,
    minterConfigPda,
    tokenAclMintConfigPda,
    ...aclConfig,
  });

  console.log(`\n═══════════════════════════════════════════════════════`);
  console.log(`  ⚠ SQUADS PROPOSALS REQUIRED`);
  console.log(`  Submit the following via Squads UI to complete setup.`);
  console.log(`═══════════════════════════════════════════════════════`);
  console.log(`\n⚠ Minting will be limited to 0 until this is executed:`);

  await printSetDailyLimitProposal({ rpc, mint, cluster, decimals, minterDailyLimit, keypair: args.keypair, multisigPubkey: args.multisigPubkey, vaultIndex: args.vaultIndex });

  console.log(`\n═══════════════════════════════════════════════════════`);
  console.log(`  ✓ Token setup complete!`);
  console.log(`  Output: ${outPath}`);
  console.log(`  Mint: ${mint}`);
  console.log(`═══════════════════════════════════════════════════════\n`);
};

const {
  values: {
    cluster,
    keypair,
    symbol,
    name,
    uri,
    decimals,
    'minter-daily-limit': minterDailyLimit,
    'permanent-delegate': permanentDelegate,
    'metadata-pointer-authority': metadataPointerAuthority,
    'metadata-update-authority': metadataUpdateAuthority,
    'pause-authority': pauseAuthority,
    'mint-close-authority': mintCloseAuthority,
    'token-acl-authority': tokenAclAuthority,
    'multisig-pubkey': multisigPubkey,
    'vault-index': vaultIndexStr,
  },
} = parseArgs({
  options: {
    cluster: { type: 'string', default: 'devnet' },
    keypair: { type: 'string' },
    symbol: { type: 'string' },
    name: { type: 'string' },
    uri: { type: 'string' },
    decimals: { type: 'string', default: '6' },
    'minter-daily-limit': { type: 'string' },
    'permanent-delegate': { type: 'string' },
    'metadata-pointer-authority': { type: 'string' },
    'metadata-update-authority': { type: 'string' },
    'pause-authority': { type: 'string' },
    'mint-close-authority': { type: 'string' },
    'token-acl-authority': { type: 'string' },
    'multisig-pubkey': { type: 'string' },
    'vault-index': { type: 'string', default: '0' },
  },
  strict: true,
});

if (
  !keypair ||
  !symbol ||
  !name ||
  !uri ||
  !minterDailyLimit ||
  !permanentDelegate ||
  !metadataPointerAuthority ||
  !metadataUpdateAuthority ||
  !pauseAuthority ||
  !mintCloseAuthority ||
  !tokenAclAuthority
) {
  console.error('Usage: pnpm tsx scripts/setup-token.ts \\');
  console.error('  --cluster devnet \\');
  console.error('  --keypair ./deployer.json \\');
  console.error('  --symbol EUTBL \\');
  console.error('  --name "EU T-Bill Token" \\');
  console.error('  --uri https://spiko.finance/eutbl \\');
  console.error('  --decimals 6 \\');
  console.error('  --minter-daily-limit 1000000 \\');
  console.error('  --permanent-delegate <pubkey> \\');
  console.error('  --metadata-pointer-authority <pubkey> \\');
  console.error('  --metadata-update-authority <pubkey> \\');
  console.error('  --pause-authority <pubkey> \\');
  console.error('  --mint-close-authority <pubkey> \\');
  console.error('  --token-acl-authority <pubkey> \\');
  console.error('  --multisig-pubkey <pubkey> (optional, devnet only) \\');
  console.error('  --vault-index 0 (optional, devnet only)');
  process.exit(1);
}

main({
  cluster,
  keypair,
  symbol,
  name,
  uri,
  decimals: Number(decimals),
  minterDailyLimit: Number(minterDailyLimit),
  permanentDelegate,
  metadataPointerAuthority,
  metadataUpdateAuthority,
  pauseAuthority,
  mintCloseAuthority,
  tokenAclAuthority,
  multisigPubkey,
  vaultIndex: Number(vaultIndexStr),
}).catch((e) => {
  console.error(e);
  process.exit(1);
});
