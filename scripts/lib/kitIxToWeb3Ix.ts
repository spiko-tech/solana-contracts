/**
 * Converts a @solana/kit Instruction to a @solana/web3.js TransactionInstruction.
 */
import { PublicKey, TransactionInstruction } from '@solana/web3.js';
import { AccountRole, isSignerRole, isWritableRole } from '@solana/kit';

export type KitInstruction = {
  programAddress: string;
  accounts?: readonly { address: string; role: AccountRole }[];
  data?: Uint8Array | ArrayLike<number>;
};

export function kitIxToWeb3Ix(ix: KitInstruction): TransactionInstruction {
  return new TransactionInstruction({
    programId: new PublicKey(ix.programAddress),
    keys: (ix.accounts ?? []).map((a) => ({
      pubkey: new PublicKey(a.address),
      isSigner: isSignerRole(a.role),
      isWritable: isWritableRole(a.role),
    })),
    data: Buffer.from(ix.data ?? new Uint8Array()),
  });
}
