import { AccountRole, getBase58Decoder } from '@solana/kit';

const formatAccount = (acc: { address: string; role: number }, idx: number): string => {
  const writable = acc.role === AccountRole.WRITABLE_SIGNER || acc.role === AccountRole.WRITABLE;
  const signer = acc.role === AccountRole.WRITABLE_SIGNER || acc.role === AccountRole.READONLY_SIGNER;
  const flags = [signer ? 'signer' : '', writable ? 'writable' : ''].filter(Boolean).join(', ');
  return `  [${idx}] ${acc.address}  (${flags || 'readonly'})`;
};

export const printInstruction = (label: string, ix: any) => {
  console.log(`\n── ${label} ──`);
  console.log(`Program:  ${ix.programAddress}`);
  console.log(`Accounts:`);
  ix.accounts.forEach((acc: any, i: number) => {
    console.log(formatAccount(acc, i));
  });
  const dataHex = Buffer.from(ix.data).toString('hex');
  const dataBase58 = getBase58Decoder().decode(ix.data);
  console.log(`Data (hex):    ${dataHex}`);
  console.log(`Data (base58): ${dataBase58}`);
};
