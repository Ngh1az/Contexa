// Pure resolution logic kept separate from the DOM-touching hook so it's
// unit-testable without mounting React or jsdom (vitest.config.ts runs
// environment: "node" — same separation overlayState.ts uses for the
// overlay's own reducer).
import { useCallback, useEffect, useState } from "react";

export type ThemePreference = "system" | "light" | "dark";
export type ResolvedTheme = "light" | "dark";

const STORAGE_KEY = "contexa-theme";

export function resolveTheme(preference: ThemePreference, systemPrefersDark: boolean): ResolvedTheme {
  if (preference === "system") return systemPrefersDark ? "dark" : "light";
  return preference;
}

export function readStoredPreference(getItem: (key: string) => string | null): ThemePreference {
  const stored = getItem(STORAGE_KEY);
  return stored === "light" || stored === "dark" ? stored : "system";
}

function systemPrefersDark(): boolean {
  return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

export function useTheme() {
  const [preference, setPreferenceState] = useState<ThemePreference>(() =>
    readStoredPreference((key) => window.localStorage.getItem(key)),
  );
  const [resolved, setResolved] = useState<ResolvedTheme>(() => resolveTheme(preference, systemPrefersDark()));

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", resolved);
  }, [resolved]);

  useEffect(() => {
    if (preference !== "system") {
      setResolved(resolveTheme(preference, false));
      return;
    }
    const mql = window.matchMedia("(prefers-color-scheme: dark)");
    const update = () => setResolved(resolveTheme("system", mql.matches));
    update();
    mql.addEventListener("change", update);
    return () => mql.removeEventListener("change", update);
  }, [preference]);

  const setPreference = useCallback((next: ThemePreference) => {
    setPreferenceState(next);
    window.localStorage.setItem(STORAGE_KEY, next);
  }, []);

  return { preference, resolved, setPreference };
}
