import React, {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useSyncExternalStore,
} from "react";
import { en } from "./locales/en";
import { zh } from "./locales/zh";
import { es } from "./locales/es";
import { fr } from "./locales/fr";
import { hi } from "./locales/hi";
import type { Lang, LangOption, TranslationSchema } from "./locales/types";

export type { Lang, LangOption, TranslationSchema };

type AmberWindow = Window & {
  __AMBER_INITIAL_LANG__?: string;
};

const LANG_STORAGE_KEY = "amber_lang";

export const SUPPORTED_LANGS: readonly LangOption[] = [
  { code: "en", label: "English", nativeLabel: "English", flag: "🇬🇧" },
  { code: "zh", label: "Chinese", nativeLabel: "简体中文", flag: "🇨🇳" },
  { code: "es", label: "Spanish", nativeLabel: "Español", flag: "🇪🇸" },
  { code: "fr", label: "French", nativeLabel: "Français", flag: "🇫🇷" },
  { code: "hi", label: "Hindi", nativeLabel: "हिन्दी", flag: "🇮🇳" },
] as const;

const translations: Record<Lang, TranslationSchema> = {
  en,
  zh,
  es,
  fr,
  hi,
};

export type LangContextValue = {
  lang: Lang;
  setLang: (lang: Lang) => void;
  toggle: () => void;
  copy: TranslationSchema;
  languages: readonly LangOption[];
};

function isLang(value: string | null | undefined): value is Lang {
  return (
    value === "en" ||
    value === "zh" ||
    value === "es" ||
    value === "fr" ||
    value === "hi"
  );
}

function amberWindow(): AmberWindow | undefined {
  if (typeof window === "undefined") return undefined;
  return window as AmberWindow;
}

// Hook/helpers colocated with LangProvider.
// eslint-disable-next-line react-refresh/only-export-components
export function resolveInitialLanguage(): Lang {
  const w = amberWindow();
  if (!w) return "en";

  try {
    const stored = w.localStorage.getItem(LANG_STORAGE_KEY);
    if (isLang(stored)) return stored;
  } catch {
    /* private mode */
  }

  const injected = w.__AMBER_INITIAL_LANG__;
  if (isLang(injected)) return injected;

  try {
    const navLangs = w.navigator.languages || [w.navigator.language || ""];
    for (const bLang of navLangs) {
      if (!bLang) continue;
      const prefix = bLang.toLowerCase().split("-")[0];
      if (isLang(prefix)) return prefix;
    }
  } catch {
    /* private mode */
  }

  return "en";
}

const listeners = new Set<() => void>();

// Snapshot source of truth. localStorage is persistence only — reading it in
// getSnapshot made hydrateRoot keep the injected boot language until refresh.
let currentLang: Lang = "en";

function emitLang() {
  for (const listener of listeners) listener();
}

function subscribeLang(listener: () => void) {
  listeners.add(listener);
  const onStorage = (event: StorageEvent) => {
    if (event.key !== LANG_STORAGE_KEY || !isLang(event.newValue)) {
      return;
    }
    currentLang = event.newValue;
    listener();
  };
  const w = amberWindow();
  w?.addEventListener("storage", onStorage);
  return () => {
    listeners.delete(listener);
    w?.removeEventListener("storage", onStorage);
  };
}

function getClientLang(): Lang {
  return currentLang;
}

function getServerLang(): Lang {
  return "en";
}

function persistLang(nextLang: Lang) {
  const w = amberWindow();
  if (!w) return;
  try {
    w.localStorage.setItem(LANG_STORAGE_KEY, nextLang);
  } catch {
    /* private mode */
  }
  w.__AMBER_INITIAL_LANG__ = nextLang;
  w.document.documentElement.lang = nextLang;
}

function applyLang(nextLang: Lang) {
  if (currentLang === nextLang) return;
  currentLang = nextLang;
  persistLang(nextLang);
  emitLang();
}

if (typeof window !== "undefined") {
  currentLang = resolveInitialLanguage();
}

const LangContext = createContext<LangContextValue | null>(null);

export function LangProvider({ children }: { children: React.ReactNode }) {
  const lang = useSyncExternalStore(
    subscribeLang,
    getClientLang,
    getServerLang,
  );

  const setLang = useCallback((nextLang: Lang) => {
    applyLang(nextLang);
  }, []);

  const value = useMemo<LangContextValue>(() => {
    const nextIdx =
      (SUPPORTED_LANGS.findIndex((item) => item.code === lang) + 1) %
      SUPPORTED_LANGS.length;
    return {
      lang,
      setLang,
      toggle: () => setLang(SUPPORTED_LANGS[nextIdx].code),
      copy: translations[lang],
      languages: SUPPORTED_LANGS,
    };
  }, [lang, setLang]);

  return <LangContext.Provider value={value}>{children}</LangContext.Provider>;
}

// eslint-disable-next-line react-refresh/only-export-components
export function useLang() {
  const ctx = useContext(LangContext);
  if (!ctx) {
    throw new Error("useLang must be used within LangProvider");
  }
  return ctx;
}
