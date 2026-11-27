/**
 * Page Visibility helpers shared by the polling hooks.
 *
 * Both helpers are SSR-safe: when `document` is unavailable the page is
 * treated as visible and subscribing is a no-op.
 */

/** True when the browser tab is currently in the background. */
export function isDocumentHidden(): boolean {
  return typeof document !== 'undefined' && document.visibilityState === 'hidden';
}

/**
 * Subscribes to `visibilitychange` events. `onChange` receives `true` when the
 * tab becomes hidden and `false` when it becomes visible again.
 *
 * @returns An unsubscribe function.
 */
export function subscribeToVisibility(onChange: (hidden: boolean) => void): () => void {
  if (typeof document === 'undefined') return () => {};
  const handler = () => onChange(isDocumentHidden());
  document.addEventListener('visibilitychange', handler);
  return () => document.removeEventListener('visibilitychange', handler);
}
