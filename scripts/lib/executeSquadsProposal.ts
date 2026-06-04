/**
 * Creates a Squads V4 vault transaction, approves it, and executes it using the SDK.
 * Intended for devnet usage where the keypair is a multisig member with sufficient threshold.
 */
import fs from 'fs';
import { Connection, Keypair, PublicKey, Signer, TransactionMessage } from '@solana/web3.js';
import * as multisig from '@sqds/multisig';
import { kitIxToWeb3Ix } from './kitIxToWeb3Ix.js';
import type { KitInstruction } from './kitIxToWeb3Ix.js';

function loadWeb3Keypair(filePath: string): Keypair {
  const bytes = new Uint8Array(JSON.parse(fs.readFileSync(filePath, 'utf-8')));
  return Keypair.fromSecretKey(bytes);
}

export const executeSquadsProposal = async (opts: {
  rpcUrl: string;
  keypairPath: string;
  multisigPubkey: string;
  vaultIndex: number;
  instructions: KitInstruction[];
  label?: string;
}) => {
  const { rpcUrl, keypairPath, multisigPubkey, vaultIndex, instructions, label } = opts;
  const connection = new Connection(rpcUrl, 'confirmed');
  const feePayer = loadWeb3Keypair(keypairPath) as unknown as Signer & Keypair;
  const multisigPda = new PublicKey(multisigPubkey);

  // Get vault PDA
  const [vaultPda] = multisig.getVaultPda({ multisigPda, index: vaultIndex });

  // Convert kit instructions to web3.js
  const web3Ixs = instructions.map(kitIxToWeb3Ix);

  // Build TransactionMessage
  const message = new TransactionMessage({
    payerKey: vaultPda,
    recentBlockhash: (await connection.getLatestBlockhash()).blockhash,
    instructions: web3Ixs,
  });

  // Get current transaction index
  const multisigAccount = await multisig.accounts.Multisig.fromAccountAddress(connection, multisigPda);
  const transactionIndex = BigInt(Number(multisigAccount.transactionIndex)) + BigInt(1);

  console.log(`\n── Squads SDK: ${label ?? 'Vault Transaction'} ──`);
  console.log(`  Multisig: ${multisigPubkey}`);
  console.log(`  Vault:    ${vaultPda.toBase58()}`);
  console.log(`  Vault index: ${vaultIndex}`);
  console.log(`  Transaction index: ${transactionIndex}`);

  // 1. Create vault transaction
  console.log(`  Creating vault transaction...`);
  const sig1 = await multisig.rpc.vaultTransactionCreate({
    connection,
    feePayer,
    multisigPda,
    transactionIndex,
    creator: feePayer.publicKey,
    vaultIndex,
    ephemeralSigners: 0,
    transactionMessage: message,
  });
  await connection.confirmTransaction(sig1, 'confirmed');

  // 2. Create proposal
  console.log(`  Creating proposal...`);
  const sig2 = await multisig.rpc.proposalCreate({
    connection,
    feePayer,
    creator: feePayer,
    multisigPda,
    transactionIndex,
    isDraft: false,
  });
  await connection.confirmTransaction(sig2, 'confirmed');

  // 3. Approve proposal
  console.log(`  Approving proposal...`);
  const sig3 = await multisig.rpc.proposalApprove({
    connection,
    feePayer,
    multisigPda,
    transactionIndex,
    member: feePayer,
  });
  await connection.confirmTransaction(sig3, 'confirmed');

  // 4. Execute vault transaction
  console.log(`  Executing vault transaction...`);
  await multisig.rpc.vaultTransactionExecute({
    connection,
    feePayer,
    multisigPda,
    transactionIndex,
    member: feePayer.publicKey,
  });

  console.log(`  Done.`);
};
