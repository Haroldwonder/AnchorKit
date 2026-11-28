import { createContext, useContext, useMemo, type ReactNode } from "react";
import { Contract, rpc } from "@stellar/stellar-sdk";

export interface AnchorKitConfig {
  rpcUrl: string;
  contractId: string;
  networkPassphrase: string;
}

export interface AnchorKitClient {
  config: AnchorKitConfig;
  contract: Contract;
  server: rpc.Server;
}

const AnchorKitContext = createContext<AnchorKitClient | null>(null);

export interface AnchorKitProviderProps extends AnchorKitConfig {
  children: ReactNode;
}

export function AnchorKitProvider({
  rpcUrl,
  contractId,
  networkPassphrase,
  children,
}: AnchorKitProviderProps) {
  const client = useMemo<AnchorKitClient>(() => {
    const server = new rpc.Server(rpcUrl);
    const contract = new Contract(contractId);
    return {
      config: { rpcUrl, contractId, networkPassphrase },
      contract,
      server,
    };
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

export interface HealthScore {
  score: number;
  updatedAt: number;
}

export type GetHealthScore = (client: AnchorKitClient) => Promise<HealthScore>;

export interface UseHealthScoreOptions {
  client?: AnchorKitClient;
  getHealthScore?: GetHealthScore;
}

export interface UseHealthScoreResult {
  data: HealthScore | null;
  loading: boolean;
  error: Error | null;
}

const defaultGetHealthScore: GetHealthScore = async (client) => {
  const result = await client.server.getHealth();
  return {
    score: Number(result?.score ?? 0),
    updatedAt: Date.now(),
  };
};

export function useHealthScore(
  options: UseHealthScoreOptions = {},
): UseHealthScoreResult {
  const providerClient = useContext(AnchorKitContext);
  const client = options.client ?? providerClient;
  const getHealthScore = options.getHealthScore ?? defaultGetHealthScore;

  const [data, setData] = useState<HealthScore | null>(null);
  const [loading, setLoading] = useState<boolean>(false);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    if (!client) {
      setError(
        new Error(
          "useHealthScore requires a client or an <AnchorKitProvider>",
        ),
      );
      return;
    }

    let cancelled = false;
    setLoading(true);
    setError(null);

    getHealthScore(client)
      .then((score) => {
        if (!cancelled) setData(score);
      })
      .catch((err: unknown) => {
        if (!cancelled) {
          setError(err instanceof Error ? err : new Error(String(err)));
        }
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [client, getHealthScore]);

  return { data, loading, error };
}
