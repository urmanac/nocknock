/**
 * Zero-custody WalletConnect bridge (Golden Path, v0.2.0).
 *
 * Uses the official @walletconnect/ethereum-provider SDK (QR modal + deep links
 * to mobile wallets such as the Uniswap app). NockNock never holds keys: the
 * sovereign wallet signs in the paired wallet app with biometrics; this module
 * only proposes pre-built transactions from the worker payload engine.
 *
 * Requires a free Reown project id (https://cloud.reown.com) provided via
 * VITE_WALLETCONNECT_PROJECT_ID. Without it the connect surface degrades to an
 * explicit setup hint — everything else in the dashboard keeps working.
 */
import { EthereumProvider } from '@walletconnect/ethereum-provider';

type EthereumProviderInstance = InstanceType<typeof EthereumProvider>;

const PROJECT_ID: string = import.meta.env.VITE_WALLETCONNECT_PROJECT_ID || '';
const BASE_CHAIN_ID = 8453; // Base mainnet

let providerPromise: Promise<EthereumProviderInstance> | null = null;

export const walletConnectConfigured = (): boolean => PROJECT_ID.trim().length > 0;

export function getProvider(): Promise<EthereumProviderInstance> {
  if (!walletConnectConfigured()) {
    return Promise.reject(
      new Error(
        'WalletConnect is not configured. Set VITE_WALLETCONNECT_PROJECT_ID (free Reown project id) and rebuild.',
      ),
    );
  }
  if (!providerPromise) {
    providerPromise = EthereumProvider.init({
      projectId: PROJECT_ID.trim(),
      chains: [BASE_CHAIN_ID],
      showQrModal: true,
      metadata: {
        name: 'NockNock',
        description: 'Sovereign Asset State Room - Golden Path execution',
        url: typeof window !== 'undefined' ? window.location.origin : 'https://nocknock.xyz',
        icons: [],
      },
    });
  }
  return providerPromise;
}

/** Pair (or resume) a session; resolves with the connected account. */
export async function connectWallet(): Promise<string> {
  const provider = await getProvider();
  if (!provider.session) {
    await provider.connect();
  }
  const account: string | undefined = provider.accounts?.[0];
  if (!account) throw new Error('WalletConnect session has no account');
  return account;
}

export async function disconnectWallet(): Promise<void> {
  const provider = await getProvider();
  if (provider.session) {
    await provider.disconnect();
  }
}

export function connectedAccount(provider: EthereumProviderInstance): string | undefined {
  return provider.accounts?.[0];
}

/**
 * Dispatch one pre-built transaction to the paired wallet for biometric signing.
 * Refuses to send if the paired wallet is not the sovereign wallet the payload
 * was built for (zero-custody: wrong-wallet proposals never reach the signer).
 */
export async function sendClaimTransaction(
  tx: { to: string; data: string; value: string },
  sovereignWallet: string,
): Promise<string> {
  const provider = await getProvider();
  if (!provider.session) throw new Error('No paired wallet session - connect first');
  const account: string | undefined = provider.accounts?.[0];
  if (!account || account.toLowerCase() !== sovereignWallet.toLowerCase()) {
    throw new Error(
      `Paired wallet ${account ?? 'unknown'} is not the sovereign wallet ${sovereignWallet}. Disconnect and pair the wallet that holds the keys.`,
    );
  }
  const hash: string = await provider.request({
    method: 'eth_sendTransaction',
    params: [{ from: account, to: tx.to, data: tx.data, value: tx.value || '0x0' }],
  });
  return hash;
}
