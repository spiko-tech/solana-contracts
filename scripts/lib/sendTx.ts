import {
  type KeyPairSigner,
  type Rpc,
  type SolanaRpcApi,
  type RpcSubscriptions,
  type SolanaRpcSubscriptionsApi,
  pipe,
  createTransactionMessage,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  appendTransactionMessageInstructions,
  signTransactionMessageWithSigners,
  sendAndConfirmTransactionFactory,
  getSignatureFromTransaction,
} from '@solana/kit';

export const sendTx = async (
  rpc: Rpc<SolanaRpcApi>,
  rpcSubscriptions: RpcSubscriptions<SolanaRpcSubscriptionsApi>,
  payer: KeyPairSigner,
  instructions: Parameters<typeof appendTransactionMessageInstructions>[0],
  label: string,
): Promise<string> => {
  const { value: latestBlockhash } = await rpc.getLatestBlockhash().send();

  const txMessage = pipe(
    createTransactionMessage({ version: 0 }),
    (tx) => setTransactionMessageFeePayerSigner(payer, tx),
    (tx) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, tx),
    (tx) => appendTransactionMessageInstructions(instructions, tx),
  );

  const signedTx = await signTransactionMessageWithSigners(txMessage);
  const sendAndConfirm = sendAndConfirmTransactionFactory({
    rpc,
    rpcSubscriptions,
  });
  await sendAndConfirm(signedTx as any, { commitment: 'confirmed' });

  const sig = getSignatureFromTransaction(signedTx);
  console.log(`  ✓ ${label}: ${sig}`);
  await new Promise((r) => setTimeout(r, 1000));
  return sig;
};
