import React, { useState, useEffect } from 'react';
import './ApiRequestPanel.css';

const HISTORY_KEY = 'anchorkit_api_history';
const HISTORY_MAX = 20;

/** Key names (matched case-insensitively, as a substring) redacted from history by default. */
const DEFAULT_SENSITIVE_FIELDS = [
  'token',
  'jwt',
  'authorization',
  'password',
  'secret',
  'apikey',
  'api_key',
  'privatekey',
  'private_key',
  'seed',
  'mnemonic',
  'signature',
  'ssn',
  'passport',
  'id_number',
  'idnumber',
  'dob',
  'date_of_birth',
];

const REDACTED = '[REDACTED]';

export interface HistoryEntry {
  id: string;
  timestamp: number;
  endpoint: string;
  method: string;
  requestBody?: Record<string, any> | string;
  response?: Record<string, any> | string;
  error?: string;
}

function redactValue(value: any, fields: string[]): any {
  if (Array.isArray(value)) return value.map((v) => redactValue(v, fields));
  if (value && typeof value === 'object') {
    const out: Record<string, any> = {};
    for (const [key, val] of Object.entries(value)) {
      const isSensitive = fields.some((f) => key.toLowerCase().includes(f.toLowerCase()));
      out[key] = isSensitive ? REDACTED : redactValue(val, fields);
    }
    return out;
  }
  return value;
}

function redactEntry(entry: HistoryEntry, fields: string[]): HistoryEntry {
  if (fields.length === 0) return entry;
  return {
    ...entry,
    requestBody:
      entry.requestBody && typeof entry.requestBody === 'object'
        ? redactValue(entry.requestBody, fields)
        : entry.requestBody,
    response:
      entry.response && typeof entry.response === 'object'
        ? redactValue(entry.response, fields)
        : entry.response,
  };
}

function loadHistory(): HistoryEntry[] {
  try {
    return JSON.parse(localStorage.getItem(HISTORY_KEY) ?? '[]');
  } catch {
    return [];
  }
}

function saveHistory(entries: HistoryEntry[]): void {
  try {
    localStorage.setItem(HISTORY_KEY, JSON.stringify(entries.slice(0, HISTORY_MAX)));
  } catch {
    // localStorage may be unavailable (SSR, private mode quota)
  }
}

export interface ApiRequestPanelProps {
  endpoint: string;
  method?: 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH';
  requestBody?: Record<string, any> | string;
  response?: Record<string, any> | string;
  headers?: Record<string, string>;
  isLoading?: boolean;
  error?: string;
  editable?: boolean;
  onSubmit?: (body: Record<string, any> | string) => void;
  /**
   * Persist request/response history to localStorage and show a history panel.
   *
   * SECURITY: entries are written to localStorage in plaintext by default, which is
   * readable by any script on the page (XSS amplification) and persists across
   * sessions on shared/public machines. If the endpoint/body/response you pass in may
   * carry auth tokens, signed XDR, or KYC PII, keep this disabled, or provide
   * `sensitiveFields`/`redactHistoryEntry` to strip that data before it is persisted.
   *
   * Default: false
   */
  persistHistory?: boolean;
  /**
   * Object key names (matched case-insensitively, as a substring, recursively) whose
   * values are replaced with '[REDACTED]' before a history entry is persisted.
   * Ignored if `redactHistoryEntry` is provided. Default: a built-in list covering
   * common auth/PII field names (token, jwt, password, ssn, etc).
   */
  sensitiveFields?: string[];
  /**
   * Custom redaction applied to each entry before it is persisted (and stored in
   * memory) as history. Takes precedence over `sensitiveFields` when provided.
   */
  redactHistoryEntry?: (entry: HistoryEntry) => HistoryEntry;
}

export const ApiRequestPanel: React.FC<ApiRequestPanelProps> = ({
  endpoint,
  method = 'POST',
  requestBody,
  response,
  headers = {},
  isLoading = false,
  error,
  editable = false,
  onSubmit,
  persistHistory = false,
  sensitiveFields = DEFAULT_SENSITIVE_FIELDS,
  redactHistoryEntry,
}) => {
  const formatJson = (data: any): string => {
    if (typeof data === 'string') return data;
    return JSON.stringify(data, null, 2);
  };

  const [copiedSection, setCopiedSection] = useState<string | null>(null);
  const [editableBody, setEditableBody] = useState<string>(
    requestBody ? formatJson(requestBody) : ''
  );
  const [jsonError, setJsonError] = useState<string | null>(null);
  const [history, setHistory] = useState<HistoryEntry[]>(() =>
    persistHistory ? loadHistory() : []
  );
  const [showHistory, setShowHistory] = useState(false);

  // Persist whenever history changes
  useEffect(() => {
    if (persistHistory) saveHistory(history);
  }, [history, persistHistory]);

  // Record a completed request into history
  useEffect(() => {
    if (!persistHistory || isLoading) return;
    // Only record when we have a response or error for the current endpoint
    if (response === undefined && !error) return;
    const rawEntry: HistoryEntry = {
      id: crypto.randomUUID(),
      timestamp: Date.now(),
      endpoint,
      method,
      requestBody,
      response,
      error,
    };
    const entry = redactHistoryEntry
      ? redactHistoryEntry(rawEntry)
      : redactEntry(rawEntry, sensitiveFields);
    setHistory((prev) => [entry, ...prev].slice(0, HISTORY_MAX));
  }, [response, error, endpoint, method, requestBody, persistHistory, isLoading, redactHistoryEntry, sensitiveFields]);

  useEffect(() => {
    setEditableBody(requestBody ? formatJson(requestBody) : '');
    setJsonError(null);
  }, [requestBody]);

  const validateJson = (value: string): boolean => {
    if (!value.trim()) {
      setJsonError(null);
      return true;
    }

    try {
      JSON.parse(value);
      setJsonError(null);
      return true;
    } catch (err) {
      const error = err as Error;
      setJsonError(error.message);
      return false;
    }
  };

  const handleBodyChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const value = e.target.value;
    setEditableBody(value);
    validateJson(value);
  };

  const handleSubmit = () => {
    if (!onSubmit || jsonError) return;

    try {
      const parsedBody = editableBody.trim() ? JSON.parse(editableBody) : {};
      onSubmit(parsedBody);
    } catch (err) {
      // Should not happen as we validate before enabling submit
      console.error('Failed to parse JSON:', err);
    }
  };

  const isSubmitDisabled = !!jsonError || isLoading;

  /**
   * Wraps a value in single quotes and escapes any embedded single quotes
   * using the POSIX '\'' idiom so the result is safe to paste into any
   * POSIX shell without risk of command injection via backticks, $(), ${},
   * backslashes, or other metacharacters.
   *
   * Examples:
   *   hello world  →  'hello world'
   *   it's a test  →  'it'\''s a test'
   */
  const shellQuoteSingle = (value: string): string =>
    `'${value.replace(/'/g, "'\\''")}'`;

  const generateCurl = (): string => {
    const headerFlags = Object.entries(headers)
      .map(([key, value]) => `-H ${shellQuoteSingle(`${key}: ${value}`)}`)
      .join(' \\\n  ');

    let curl = `curl -X ${method} \\\n  ${shellQuoteSingle(endpoint)}`;

    if (headerFlags) {
      curl += ` \\\n  ${headerFlags}`;
    }

    if (requestBody && (method === 'POST' || method === 'PUT' || method === 'PATCH')) {
      const body = formatJson(requestBody);
      curl += ` \\\n  -d ${shellQuoteSingle(body)}`;
    }

    return curl;
  };

  const copyToClipboard = async (text: string, section: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopiedSection(section);
      setTimeout(() => setCopiedSection(null), 2000);
    } catch (err) {
      console.error('Failed to copy:', err);
    }
  };

  const CopyButton: React.FC<{ text: string; section: string }> = ({ text, section }) => (
    <button
      className="copy-button"
      onClick={() => copyToClipboard(text, section)}
      title={`Copy ${section}`}
    >
      {copiedSection === section ? (
        <span className="copy-icon">✓</span>
      ) : (
        <span className="copy-icon">📋</span>
      )}
    </button>
  );

  return (
    <div className="api-request-panel">
      {/* Endpoint Section */}
      <div className="panel-section endpoint-section">
        <div className="section-header">
          <span className="method-badge" data-method={method}>
            {method}
          </span>
          <h3>Endpoint</h3>
        </div>
        <div className="section-content">
          <code className="endpoint-url">{endpoint}</code>
          <CopyButton text={endpoint} section="endpoint" />
        </div>
      </div>

      {/* Request Body Section */}
      {(requestBody || editable) && (
        <div className="panel-section request-section">
          <div className="section-header">
            <h3>Request Body</h3>
            {!editable && <CopyButton text={formatJson(requestBody)} section="request" />}
          </div>
          <div className="section-content">
            {editable ? (
              <div className="editable-body-container">
                <textarea
                  className={`editable-body ${jsonError ? 'error' : ''}`}
                  value={editableBody}
                  onChange={handleBodyChange}
                  placeholder='{"key": "value"}'
                  rows={10}
                  aria-label="Request body JSON"
                  aria-invalid={!!jsonError}
                  aria-describedby={jsonError ? 'json-error' : undefined}
                />
                {jsonError && (
                  <div className="json-error" id="json-error" role="alert">
                    <span className="error-icon">⚠️</span>
                    <span>Invalid JSON: {jsonError}</span>
                  </div>
                )}
                {onSubmit && (
                  <button
                    className="submit-button"
                    onClick={handleSubmit}
                    disabled={isSubmitDisabled}
                    title={jsonError ? 'Fix JSON errors before submitting' : 'Submit request'}
                  >
                    {isLoading ? 'Submitting...' : 'Submit Request'}
                  </button>
                )}
              </div>
            ) : (
              <pre className="code-block">
                <code>{formatJson(requestBody)}</code>
              </pre>
            )}
          </div>
        </div>
      )}

      {/* Response Section */}
      <div className="panel-section response-section">
        <div className="section-header">
          <h3>Response</h3>
          {response && <CopyButton text={formatJson(response)} section="response" />}
        </div>
        <div className="section-content">
          {isLoading ? (
            <div className="skeleton-loader">
              <div className="skeleton-line"></div>
              <div className="skeleton-line"></div>
              <div className="skeleton-line short"></div>
            </div>
          ) : error ? (
            <div className="error-message">
              <span className="error-icon">⚠️</span>
              <span>{error}</span>
            </div>
          ) : response ? (
            <pre className="code-block">
              <code>{formatJson(response)}</code>
            </pre>
          ) : (
            <div className="empty-state">No response yet</div>
          )}
        </div>
      </div>

      {/* cURL Command Section */}
      <div className="panel-section curl-section">
        <div className="section-header">
          <h3>cURL Command</h3>
          <CopyButton text={generateCurl()} section="curl" />
        </div>
        <div className="section-content">
          <pre className="code-block curl-block">
            <code>{generateCurl()}</code>
          </pre>
        </div>
      </div>

      {/* History Section */}
      {persistHistory && (
        <div className="panel-section history-section">
          <div className="section-header">
            <h3>History</h3>
            <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
              {history.length > 0 && (
                <button
                  className="copy-button"
                  onClick={() => { setHistory([]); saveHistory([]); }}
                  title="Clear history"
                >
                  <span className="copy-icon">🗑️</span>
                </button>
              )}
              <button
                className="copy-button"
                onClick={() => setShowHistory((v) => !v)}
                title={showHistory ? 'Hide history' : 'Show history'}
              >
                <span className="copy-icon">{showHistory ? '▲' : '▼'}</span>
              </button>
            </div>
          </div>
          {showHistory && (
            <div className="section-content">
              {history.length === 0 ? (
                <div className="empty-state">No history yet</div>
              ) : (
                <ul className="history-list" aria-label="Request history">
                  {history.map((entry) => (
                    <li key={entry.id} className="history-item">
                      <span className="method-badge" data-method={entry.method}>{entry.method}</span>
                      <code className="endpoint-url">{entry.endpoint}</code>
                      <span className="history-time">
                        {new Date(entry.timestamp).toLocaleTimeString()}
                      </span>
                      {entry.error && (
                        <span className="error-icon" title={entry.error}>⚠️</span>
                      )}
                    </li>
                  ))}
                </ul>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
};
