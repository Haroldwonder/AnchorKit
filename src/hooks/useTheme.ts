import { useCallback, useEffect, useState } from 'react';

export type Theme = 'light' | 'dark' | 'high-contrast';

const THEME_STORAGE_KEY = 'theme';
const HIGH_CONTRAST_QUERY = '(prefers-contrast: more)';

function isTheme(value: string | null): value is Theme {
  return value === 'light' || value === 'dark' || value === 'high-contrast';
}

function getSystemTheme(): Theme {
  if (typeof window === 'undefined') {
    return 'light';
  }
  if (window.matchMedia?.(HIGH_CONTRAST_QUERY).matches) {
    return 'high-contrast';
  }
  if (window.matchMedia?.('(prefers-color-scheme: dark)').matches) {
    return 'dark';
  }
  return 'light';
}

function getStoredTheme(): Theme | null {
  if (typeof window === 'undefined') {
    return null;
  }
  const stored = window.localStorage.getItem(THEME_STORAGE_KEY);
  return isTheme(stored) ? stored : null;
}

function applyTheme(theme: Theme): void {
  if (typeof document === 'undefined') {
    return;
  }
  document.documentElement.setAttribute('data-theme', theme);
}

export function useTheme() {
  const [theme, setThemeState] = useState<Theme>(() => getStoredTheme() ?? getSystemTheme());

  useEffect(() => {
    applyTheme(theme);
  }, [theme]);

  useEffect(() => {
    if (typeof window === 'undefined' || !window.matchMedia) {
      return;
    }
    const query = window.matchMedia(HIGH_CONTRAST_QUERY);
    const handleChange = (event: MediaQueryListEvent) => {
      if (event.matches && !getStoredTheme()) {
        setThemeState('high-contrast');
      }
    };
    query.addEventListener?.('change', handleChange);
    return () => query.removeEventListener?.('change', handleChange);
  }, []);

  const setTheme = useCallback((next: Theme) => {
    setThemeState(next);
    if (typeof window !== 'undefined') {
      window.localStorage.setItem(THEME_STORAGE_KEY, next);
    }
  }, []);

  return { theme, setTheme };
}
