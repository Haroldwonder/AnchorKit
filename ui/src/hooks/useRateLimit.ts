import { createContext, useContext, useMemo, type ReactNode } from "react";
import {
  Client as AnchorKitClient,
  type ClientOptions,
} from "@anchorkit/sdk";

export interface AnchorKitProviderProps {
  rpcUrl: string;
  contractId: string;
  networkPassphrase: string;
  children?: ReactNode;
}

const AnchorKitContext = createContext<AnchorKitClient | null>(null);

export function AnchorKitProvider({
  rpcUrl,
  contractId,
  networkPassphrase,
  children,
}: AnchorKitProviderProps) {
  const client = useMemo(() => {
    const options: ClientOptions = {
      rpcUrl,
      contractId,
      networkPassphrase,
    };
    return new AnchorKitClient(options);
  }, [rpcUrl, contractId, networkPassphrase]);

  return (
    <AnchorKitContext.Provider value={client}>
      {children}
    </AnchorKitContext.Provider>
  );
}

export function useAnchorKitClient(): AnchorKitClient {
  const client = useContext(AnchorKitContext);
  if (!client) {
    throw new Error(
      "useAnchorKitClient must be used within an <AnchorKitProvider>",
    );
  }
  return client;
}

export interface RateLimitInfo {
  limit: number;
  remaining: number;
  resetAt: number;
}

export function useRateLimit(client?: AnchorKitClient) {
  const providerClient = useContext(AnchorKitContext);
  const activeClient = client ?? providerClient;

  if (!activeClient) {
    throw new Error(
      "useRateLimit requires a client argument or an <AnchorKitProvider>",
    );
  }

  return useMemo(() => {
    return {
      async getRateLimit(): Promise<RateLimitInfo> {
        return activeClient.getRateLimit();
      },
    };
  }, [activeClient]);
}
