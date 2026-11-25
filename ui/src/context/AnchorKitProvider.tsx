import React, {
  createContext,
  useContext,
  useMemo,
  type ReactNode,
} from "react";
import { Client as AnchorKitClient } from "@anchor-kit/sdk";

export interface AnchorKitProviderProps {
  rpcUrl: string;
  contractId: string;
  networkPassphrase: string;
  children?: ReactNode;
}

const AnchorKitContext = createContext<AnchorKitClient | null>(null);

/**
 * Builds a typed AnchorKit contract client once and supplies it to all UI hooks
 * via context. Hooks fall back to this client when no explicit client is passed.
 */
export function AnchorKitProvider({
  rpcUrl,
  contractId,
  networkPassphrase,
  children,
}: AnchorKitProviderProps) {
  const client = useMemo(
    () =>
      new AnchorKitClient({
        rpcUrl,
        contractId,
        networkPassphrase,
      }),
    [rpcUrl, contractId, networkPassphrase],
  );

  return (
    <AnchorKitContext.Provider value={client}>
      {children}
    </AnchorKitContext.Provider>
  );
}

/**
 * Returns the provider-supplied AnchorKit client.
 * Throws if used outside of an <AnchorKitProvider>.
 */
export function useAnchorKitClient(): AnchorKitClient {
  const client = useContext(AnchorKitContext);
  if (!client) {
    throw new Error(
      "useAnchorKitClient must be used within an <AnchorKitProvider>",
    );
  }
  return client;
}

export default AnchorKitProvider;
