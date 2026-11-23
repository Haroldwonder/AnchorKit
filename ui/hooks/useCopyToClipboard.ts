import { useState, useCallback, useEffect, useRef } from 'react';

export interface CopyToClipboardOptions {
  /**
   * Duration in milliseconds to show the "copied" state
   * @default 2000
   */
  successDuration?: number;
  
  /**
   * Callback fired when copy succeeds
   */
  onSuccess?: () => void;
  
  /**
   * Callback fired when copy fails
   */
  onError?: (error: Error) => void;
}

export interface CopyToClipboardResult {
  /**
   * The text that was last copied (or attempted to copy)
   */
  copiedText: string | null;
  
  /**
   * Whether the copy operation is currently showing success state
   */
  isCopied: boolean;
  
  /**
   * Copy text to clipboard
   */
  copy: (text: string) => Promise<boolean>;
  
  /**
   * Reset the copied state manually
   */
  reset: () => void;
}

/**
 * Hook for copying text to clipboard with success feedback
 * 
 * @example
 * ```tsx
 * const { copy, isCopied } = useCopyToClipboard();
 * 
 * <button onClick={() => copy(jsonData)}>
 *   {isCopied ? '✓ Copied' : 'Copy JSON'}
 * </button>
 * ```
 */
export function useCopyToClipboard(
  options: CopyToClipboardOptions = {}
): CopyToClipboardResult {
  const {
    successDuration = 2000,
    onSuccess,
    onError,
  } = options;

  const [copiedText, setCopiedText] = useState<string | null>(null);
  const [isCopied, setIsCopied] = useState(false);
  const [copyCount, setCopyCount] = useState(0);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Reset isCopied after successDuration whenever copy is called
  useEffect(() => {
    if (!isCopied) return;
    timerRef.current = setTimeout(() => {
      setIsCopied(false);
    }, successDuration);
    return () => {
      if (timerRef.current !== null) {
        clearTimeout(timerRef.current);
        timerRef.current = null;
      }
    };
  }, [copyCount, isCopied, successDuration]);

  const copy = useCallback(
    async (text: string): Promise<boolean> => {
      try {
        let copied = false;

        if (navigator?.clipboard?.writeText) {
          await navigator.clipboard.writeText(text);
          setCopiedText(text);
          setIsCopied(true);
          setCopyCount(c => c + 1);
          onSuccess?.();
          return true;
        }

        // Fallback to execCommand for older browsers or non-secure contexts
        if (document.execCommand) {
          const textArea = document.createElement('textarea');
          textArea.value = text;
          textArea.style.position = 'fixed';
          textArea.style.left = '-999999px';
          textArea.style.top = '-999999px';
          document.body.appendChild(textArea);
          textArea.focus();
          textArea.select();

          const successful = document.execCommand('copy');
          document.body.removeChild(textArea);

          if (successful) {
            setCopiedText(text);
            setIsCopied(true);
            setCopyCount(c => c + 1);
            onSuccess?.();
            return true;
          }
        }

        // If both methods fail
        throw new Error('Clipboard API and execCommand are not available');
      } catch (err) {
        const error = err instanceof Error ? err : new Error('Failed to copy');
        console.error('Copy failed:', error);
        onError?.(error);
        return false;
      }
    },
    [onSuccess, onError]
  );

  const reset = useCallback(() => {
    setCopiedText(null);
    setIsCopied(false);
  }, []);

  return {
    copiedText,
    isCopied,
    copy,
    reset,
  };
}

/**
 * Utility function to format JSON for copying
 */
export function formatJsonForCopy(data: any, pretty: boolean = true): string {
  if (typeof data === 'string') {
    return data;
  }
  return JSON.stringify(data, null, pretty ? 2 : 0);
}

/**
 * Utility function to generate curl command
 */
export function generateCurlCommand(options: {
  url: string;
  method?: string;
  headers?: Record<string, string>;
  body?: any;
}): string {
  const { url, method = 'GET', headers = {}, body } = options;

  let curl = `curl -X ${method} \\\n  "${url}"`;

  // Add headers
  const headerEntries = Object.entries(headers);
  if (headerEntries.length > 0) {
    const headerFlags = headerEntries
      .map(([key, value]) => `-H "${key}: ${value}"`)
      .join(' \\\n  ');
    curl += ` \\\n  ${headerFlags}`;
  }

  // Add body for POST/PUT/PATCH
  if (body && ['POST', 'PUT', 'PATCH'].includes(method.toUpperCase())) {
    const bodyStr = formatJsonForCopy(body);
    curl += ` \\\n  -d '${bodyStr}'`;
  }

  return curl;
}

/**
 * Utility function to generate install commands
 */
export function generateInstallCommand(
  packageName: string,
  packageManager: 'npm' | 'yarn' | 'pnpm' = 'npm'
): string {
  const commands = {
    npm: `npm install ${packageName}`,
    yarn: `yarn add ${packageName}`,
    pnpm: `pnpm add ${packageName}`,
  };

  return commands[packageManager];
}
