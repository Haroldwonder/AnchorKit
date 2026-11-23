import { TxStatus } from '../components/TransactionTimeline';

export type FetchStatusFn = (txId: string) => Promise<TxStatus>;

export interface TransactionTransition {
  from: TxStatus | null;
  to: TxStatus;
  at: number;
}

export interface TransactionSnapshot {
  status: TxStatus;
  transitions: TransactionTransition[];
  isTerminal: boolean;
}

const VALID_STATUSES = new Set<TxStatus>([
  'initiated',
  'awaiting_user',
  'pending',
  'processing',
  'completed',
  'failed',
  'refunded',
]);

const TERMINAL_STATUSES = new Set<TxStatus>(['completed', 'failed', 'refunded']);

export function isTerminalStatus(status: TxStatus): boolean {
  return TERMINAL_STATUSES.has(status);
}

function assertValidStatus(status: unknown): asserts status is TxStatus {
  if (typeof status !== 'string' || !VALID_STATUSES.has(status as TxStatus)) {
    throw new Error(`Invalid transaction status: ${String(status)}`);
  }
}

/**
 * Tracks a single transaction's status history by calling a fetch function
 * and recording each status transition. Designed to be used by useTransactionStatus.
 */
export class TransactionStateTracker {
  private _status: TxStatus | null = null;
  private _transitions: TransactionTransition[] = [];

  constructor(private readonly fetchStatus: FetchStatusFn) {}

  async poll(txId: string): Promise<TransactionSnapshot> {
    const newStatus = await this.fetchStatus(txId);
    assertValidStatus(newStatus);
    if (newStatus !== this._status) {
      this._transitions = [
        ...this._transitions,
        { from: this._status, to: newStatus, at: Date.now() },
      ];
      this._status = newStatus;
    }
    return this.getSnapshot();
  }

  getSnapshot(): TransactionSnapshot {
    if (this._status === null) {
      throw new Error('No status polled yet');
    }
    return {
      status: this._status,
      transitions: [...this._transitions],
      isTerminal: isTerminalStatus(this._status),
    };
  }

  reset(): void {
    this._status = null;
    this._transitions = [];
  }
}
