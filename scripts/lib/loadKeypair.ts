import { type KeyPairSigner, createKeyPairSignerFromBytes } from '@solana/kit';
import fs from 'fs';

export const loadKeypair = async (filePath: string): Promise<KeyPairSigner> => {
  const bytes = new Uint8Array(JSON.parse(fs.readFileSync(filePath, 'utf-8')));
  return createKeyPairSignerFromBytes(bytes);
};
