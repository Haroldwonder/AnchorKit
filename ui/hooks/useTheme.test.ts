import { renderHook, act } from '@testing-library/react';
import { useTheme } from './useTheme';

// ── helpers ──────────────────────────────────────────────────────────────────

type MQListener = (e: MediaQueryListEvent) => void;

function makeMockMQ(matches: boolean) {
  const listeners: MQListener[] = [];
  const mq = {
    matches,
    addEventListener: jest.fn((_: string, fn: MQListener) => listeners.push(fn)),
    removeEventListener: jest.fn((_: string, fn: MQListener) => {
      const idx = listeners.indexOf(fn);
      if (idx !== -1) listeners.splice(idx, 1);
    }),
    /** Simulate OS-level preference change */
    emit(newMatches: boolean) {
      mq.matches = newMatches;
      listeners.forEach((fn) => fn({ matches: newMatches } as MediaQueryListEvent));
    },
  };
  return mq;
}

// ── setup / teardown ─────────────────────────────────────────────────────────

let mockMQ: ReturnType<typeof makeMockMQ>;

beforeEach(() => {
  mockMQ = makeMockMQ(false); // default: light system preference
  window.matchMedia = jest.fn().mockReturnValue(mockMQ);

  // Reset DOM state
  document.documentElement.removeAttribute('class');
  document.documentElement.removeAttribute('data-theme');

  // Reset localStorage
  localStorage.clear();
});

afterEach(() => {
  jest.restoreAllMocks();
});

// ── tests ─────────────────────────────────────────────────────────────────────

describe('useTheme – initial state', () => {
  it('defaults to light when system preference is light and localStorage is empty', () => {
    const { result } = renderHook(() => useTheme());
    expect(result.current).toBe(false);
  });

  it('defaults to dark when system preference is dark and localStorage is empty', () => {
    mockMQ = makeMockMQ(true);
    window.matchMedia = jest.fn().mockReturnValue(mockMQ);

    const { result } = renderHook(() => useTheme());
    expect(result.current).toBe(true);
  });

  it('restores "dark" preference from localStorage, overriding system light preference', () => {
    localStorage.setItem('theme', 'dark');

    const { result } = renderHook(() => useTheme());
    expect(result.current).toBe(true);
  });

  it('restores "light" preference from localStorage, overriding system dark preference', () => {
    mockMQ = makeMockMQ(true);
    window.matchMedia = jest.fn().mockReturnValue(mockMQ);
    localStorage.setItem('theme', 'light');

    const { result } = renderHook(() => useTheme());
    expect(result.current).toBe(false);
  });

  it('falls back to system preference when localStorage value is unrecognised', () => {
    localStorage.setItem('theme', 'sepia'); // unrecognised
    mockMQ = makeMockMQ(true);
    window.matchMedia = jest.fn().mockReturnValue(mockMQ);

    const { result } = renderHook(() => useTheme());
    expect(result.current).toBe(true);
  });
});

describe('useTheme – CSS class on document.documentElement', () => {
  it('adds the "dark" class when theme is dark', () => {
    mockMQ = makeMockMQ(true);
    window.matchMedia = jest.fn().mockReturnValue(mockMQ);

    renderHook(() => useTheme());
    expect(document.documentElement.classList.contains('dark')).toBe(true);
  });

  it('does not add the "dark" class when theme is light', () => {
    renderHook(() => useTheme());
    expect(document.documentElement.classList.contains('dark')).toBe(false);
  });

  it('does not mutate the class when an override changes', () => {
    let override = true;
    const { rerender } = renderHook(() => useTheme(override));
    expect(document.documentElement.classList.contains('dark')).toBe(false);

    override = false;
    rerender();
    expect(document.documentElement.classList.contains('dark')).toBe(false);
  });

  it('does not let an override mutate the class owned by a non-overridden hook', () => {
    let override = false;
    const { rerender } = renderHook(() => useTheme(override));
    renderHook(() => useTheme());
    expect(document.documentElement.classList.contains('dark')).toBe(false);

    override = true;
    rerender();
    expect(document.documentElement.classList.contains('dark')).toBe(false);
  });
});

describe('useTheme – data-theme attribute on document.documentElement', () => {
  it('sets data-theme="light" when theme is light', () => {
    renderHook(() => useTheme());
    expect(document.documentElement.getAttribute('data-theme')).toBe('light');
  });

  it('sets data-theme="dark" when theme is dark', () => {
    mockMQ = makeMockMQ(true);
    window.matchMedia = jest.fn().mockReturnValue(mockMQ);

    renderHook(() => useTheme());
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
  });

  it('does not update data-theme when an override toggles', () => {
    let override = false;
    const { rerender } = renderHook(() => useTheme(override));
    expect(document.documentElement.getAttribute('data-theme')).toBeNull();

    override = true;
    rerender();
    expect(document.documentElement.getAttribute('data-theme')).toBeNull();
  });

  it('does not let an override update data-theme owned by a non-overridden hook', () => {
    let override = true;
    const { rerender } = renderHook(() => useTheme(override));
    renderHook(() => useTheme());
    expect(document.documentElement.getAttribute('data-theme')).toBe('light');

    override = false;
    rerender();
    expect(document.documentElement.getAttribute('data-theme')).toBe('light');
  });
});

describe('useTheme – localStorage persistence', () => {
  it('writes "light" to localStorage when theme is light', () => {
    renderHook(() => useTheme());
    expect(localStorage.getItem('theme')).toBe('light');
  });

  it('writes "dark" to localStorage when theme is dark', () => {
    mockMQ = makeMockMQ(true);
    window.matchMedia = jest.fn().mockReturnValue(mockMQ);

    renderHook(() => useTheme());
    expect(localStorage.getItem('theme')).toBe('dark');
  });

  it('does not persist an override', () => {
    let override = false;
    const { rerender } = renderHook(() => useTheme(override));
    expect(localStorage.getItem('theme')).toBeNull();

    override = true;
    rerender();
    expect(localStorage.getItem('theme')).toBeNull();
  });

  it('does not let an override overwrite the persisted preference', () => {
    let override = true;
    const { rerender } = renderHook(() => useTheme(override));
    localStorage.setItem('theme', 'light');

    override = false;
    rerender();
    expect(localStorage.getItem('theme')).toBe('light');
  });
});

describe('useTheme – override prop', () => {
  it('returns true when override is true regardless of system preference', () => {
    // system is light
    const { result } = renderHook(() => useTheme(true));
    expect(result.current).toBe(true);
  });

  it('returns false when override is false regardless of system preference', () => {
    mockMQ = makeMockMQ(true); // system is dark
    window.matchMedia = jest.fn().mockReturnValue(mockMQ);

    const { result } = renderHook(() => useTheme(false));
    expect(result.current).toBe(false);
  });

  it('uses system preference when override is undefined', () => {
    mockMQ = makeMockMQ(true);
    window.matchMedia = jest.fn().mockReturnValue(mockMQ);

    const { result } = renderHook(() => useTheme(undefined));
    expect(result.current).toBe(true);
  });
});

describe('useTheme – media query listener', () => {
  it('registers an event listener for prefers-color-scheme changes', () => {
    renderHook(() => useTheme());
    expect(mockMQ.addEventListener).toHaveBeenCalledWith('change', expect.any(Function));
  });

  it('updates theme when OS switches to dark', () => {
    const { result } = renderHook(() => useTheme());
    expect(result.current).toBe(false);

    act(() => mockMQ.emit(true));

    expect(result.current).toBe(true);
    expect(document.documentElement.classList.contains('dark')).toBe(true);
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
    expect(localStorage.getItem('theme')).toBe('dark');
  });

  it('updates theme when OS switches to light', () => {
    mockMQ = makeMockMQ(true);
    window.matchMedia = jest.fn().mockReturnValue(mockMQ);

    const { result } = renderHook(() => useTheme());
    expect(result.current).toBe(true);

    act(() => mockMQ.emit(false));

    expect(result.current).toBe(false);
    expect(document.documentElement.classList.contains('dark')).toBe(false);
    expect(document.documentElement.getAttribute('data-theme')).toBe('light');
    expect(localStorage.getItem('theme')).toBe('light');
  });

  it('removes the event listener on unmount', () => {
    const { unmount } = renderHook(() => useTheme());
    unmount();
    expect(mockMQ.removeEventListener).toHaveBeenCalledWith('change', expect.any(Function));
  });

  it('does not react to system preference changes when override is set', () => {
    const { result } = renderHook(() => useTheme(false));
    expect(result.current).toBe(false);

    // System flips to dark — override should win
    act(() => mockMQ.emit(true));

    expect(result.current).toBe(false);
  });
});

describe('useTheme – state isolation between tests', () => {
  it('starts fresh: no "dark" class on documentElement', () => {
    renderHook(() => useTheme());
    expect(document.documentElement.classList.contains('dark')).toBe(false);
  });

  it('starts fresh: localStorage is empty', () => {
    expect(localStorage.getItem('theme')).toBeNull();
    renderHook(() => useTheme());
    expect(localStorage.getItem('theme')).toBe('light');
  });
});
