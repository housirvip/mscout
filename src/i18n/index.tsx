import { createContext, useContext, useState, useCallback, ReactNode } from "react";
import zh from "./zh";
import en from "./en";

export type Locale = "zh" | "en";
export type MessageKey = keyof typeof zh;
type Messages = Record<MessageKey, string>;

const locales: Record<Locale, Messages> = { zh, en };

interface I18nContextType {
  locale: Locale;
  setLocale: (l: Locale) => void;
  t: (key: MessageKey, vars?: Record<string, string | number>) => string;
}

const I18nContext = createContext<I18nContextType>(null!);

export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocale] = useState<Locale>(() => {
    const stored = localStorage.getItem("mem-scanner-locale");
    if (stored === "en" || stored === "zh") return stored;
    return navigator.language.startsWith("zh") ? "zh" : "en";
  });

  const t = useCallback((key: MessageKey, vars?: Record<string, string | number>) => {
    let msg: string = locales[locale][key] ?? key;
    if (vars) {
      for (const [k, v] of Object.entries(vars)) {
        msg = msg.replaceAll(`{${k}}`, String(v));
      }
    }
    return msg;
  }, [locale]);

  const handleSetLocale = useCallback((l: Locale) => {
    setLocale(l);
    localStorage.setItem("mem-scanner-locale", l);
  }, []);

  return (
    <I18nContext.Provider value={{ locale, setLocale: handleSetLocale, t }}>
      {children}
    </I18nContext.Provider>
  );
}

export function useI18n() { return useContext(I18nContext); }
