import {
  type Rpc,
  type SolanaRpcApi,
  type RpcSubscriptions,
  type SolanaRpcSubscriptionsApi,
  createSolanaRpc,
  createSolanaRpcSubscriptions,
} from '@solana/kit';

const CLUSTER_URLS: Record<string, string> = {
  devnet: 'https://api.devnet.solana.com',
  'mainnet-beta': 'https://api.mainnet-beta.solana.com',
};

const rpcUrlToWsUrl = (rpcUrl: string): string => {
  let wsUrl = rpcUrl.replace('https://', 'wss://').replace('http://', 'ws://');
  if (wsUrl.includes('127.0.0.1:8899') || wsUrl.includes('localhost:8899')) wsUrl = wsUrl.replace(':8899', ':8900');
  return wsUrl;
};

export const getRpc = (
  cluster: string,
): {
  rpc: Rpc<SolanaRpcApi>;
  rpcSub: RpcSubscriptions<SolanaRpcSubscriptionsApi>;
} => {
  const url = CLUSTER_URLS[cluster];
  if (!url) throw new Error(`Unknown cluster: ${cluster}`);
  return { rpc: createSolanaRpc(url), rpcSub: createSolanaRpcSubscriptions(rpcUrlToWsUrl(url)) };
};
