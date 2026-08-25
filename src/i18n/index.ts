import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import enUs from "./locales/en-US.json";
import zhCn from "./locales/zh-CN.json";
import { effectiveUiLanguage, systemLanguage } from "../shared/settings";
import type { SupportedLanguage, UiLanguage } from "../shared/types";

export const resources = {
  "zh-CN": { translation: zhCn },
  "en-US": { translation: enUs },
} as const;

void i18n.use(initReactI18next).init({
  resources,
  lng: systemLanguage(),
  fallbackLng: "en-US",
  supportedLngs: ["zh-CN", "en-US"],
  interpolation: { escapeValue: false },
  returnNull: false,
});

export function applyUiLanguage(language: UiLanguage): SupportedLanguage {
  const resolved = effectiveUiLanguage(language);
  if (i18n.resolvedLanguage !== resolved) void i18n.changeLanguage(resolved);
  document.documentElement.lang = resolved;
  return resolved;
}

export default i18n;
