import { useCallback, useEffect, useState } from 'react';

export type Theme = 'light' | 'dark' | 'high-contrast';

const THEME_STORAGE_KEY = 'app-theme';

const isTheme = (value: string | null): value is Theme =>
  value === 'light' || value === 'dark' || value === 'high-contrast';

const getInitialTheme = (): Theme => {
  if (typeof window === 'undefined') {
    return 'light';
  }

  const stored = window.localStorage.getItem(THEME_STORAGE_KEY);
  if (isTheme(stored)) {
    return stored;
  }

  if (window.matchMedia?.('(prefers-contrast: more)').matches) {
    return 'high-contrast';
  }

  if (window.matchMedia?.('(prefers-color-scheme: dark)').matches) {
    return 'dark';
  }

  return 'light';
};

const applyTheme = (theme: Theme) => {
  if (typeof document === 'undefined') {
    return;
  }

  document.documentElement.setAttribute('data-theme', theme);
};

export const useTheme = () => {
  const [theme, setThemeState] = useState<Theme>(getInitialTheme);

  useEffect(() => {
    applyTheme(theme);
  }, [theme]);

  useEffect(() => {
    if (typeof window === 'undefined' || !window.matchMedia) {
      return;
    }

    const query = window.matchMedia('(prefers-contrast: more)');
    const handleChange = (event: MediaQueryListEvent) => {
      if (event.matches) {
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
};
