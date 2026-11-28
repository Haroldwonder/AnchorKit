import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from "react";

export type Theme = "light" | "dark" | "high-contrast";

const THEME_STORAGE_KEY = "theme";

interface ThemeContextValue {
  theme: Theme;
  setTheme: (theme: Theme) => void;
  toggleTheme: () => void;
}

const ThemeContext = createContext<ThemeContextValue | undefined>(undefined);

function isTheme(value: string | null): value is Theme {
  return value === "light" || value === "dark" || value === "high-contrast";
}

function getInitialTheme(): Theme {
  if (typeof window === "undefined") {
    return "light";
  }

  const stored = window.localStorage.getItem(THEME_STORAGE_KEY);
  if (isTheme(stored)) {
    return stored;
  }

  if (window.matchMedia?.("(prefers-contrast: more)").matches) {
    return "high-contrast";
  }

  if (window.matchMedia?.("(prefers-color-scheme: dark)").matches) {
    return "dark";
  }

  return "light";
}

function applyTheme(theme: Theme) {
  if (typeof document === "undefined") {
    return;
  }

  document.documentElement.setAttribute("data-theme", theme);
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [theme, setThemeState] = useState<Theme>(getInitialTheme);

  useEffect(() => {
    applyTheme(theme);
    window.localStorage.setItem(THEME_STORAGE_KEY, theme);
  }, [theme]);

  useEffect(() => {
    if (typeof window === "undefined" || !window.matchMedia) {
      return;
    }

    const query = window.matchMedia("(prefers-contrast: more)");
    const handleChange = (event: MediaQueryListEvent) => {
      if (event.matches) {
        setThemeState("high-contrast");
      }
    };

    query.addEventListener("change", handleChange);
    return () => query.removeEventListener("change", handleChange);
  }, []);

  const value = useMemo<ThemeContextValue>(
    () => ({
      theme,
      setTheme: setThemeState,
      toggleTheme: () =>
        setThemeState((current) =>
          current === "light" ? "dark" : current === "dark" ? "high-contrast" : "light",
        ),
    }),
    [theme],
  );

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}

export function useTheme(): ThemeContextValue {
  const context = useContext(ThemeContext);
  if (!context) {
    throw new Error("useTheme must be used within a ThemeProvider");
  }
  return context;
}
