// Language catalogue for Mr. Day One.
//
// Keep the product language surface intentionally small and fully supportable:
// Chinese, English, Japanese, Korean, German, Spanish, Portuguese and Russian.
// Region-specific saved values such as en-US/de-DE/pt-BR are accepted, but they
// are coerced to the supported base language before persistence or i18n loading.
export const GLOBAL_LANGUAGE_TAGS = Object.freeze([
  "zh-CN",
  "en",
  "ja",
  "ko",
  "de",
  "es",
  "pt",
  "ru",
]);

const LABEL_OVERRIDES = {
  "zh-CN": "简体中文",
  en: "English",
  ja: "日本語",
  ko: "한국어",
  de: "Deutsch",
  es: "Español",
  pt: "Português",
  ru: "Русский",
};

export const SUPPORTED_LANGUAGE_TAGS = GLOBAL_LANGUAGE_TAGS;

const SUPPORTED_LANGUAGE_SET = new Set(SUPPORTED_LANGUAGE_TAGS);
const BASE_LANGUAGE_TO_SUPPORTED_TAG = Object.freeze({
  zh: "zh-CN",
  en: "en",
  ja: "ja",
  ko: "ko",
  de: "de",
  es: "es",
  pt: "pt",
  ru: "ru",
});

export function normalizeLocaleTag(locale, fallback = "zh-CN") {
  const raw = String(locale || "").trim().replace(/_/g, "-");
  if (!raw) return fallback;
  try {
    return Intl.getCanonicalLocales(raw)[0] || fallback;
  } catch {
    return fallback;
  }
}

export function localeLanguageCode(locale, fallback = "zh") {
  const tag = normalizeLocaleTag(locale, fallback || "zh-CN");
  return String(tag || fallback).split("-")[0].toLowerCase();
}

export function coerceSupportedLocale(locale, fallback = "zh-CN") {
  const fallbackTag = SUPPORTED_LANGUAGE_SET.has(normalizeLocaleTag(fallback, "zh-CN"))
    ? normalizeLocaleTag(fallback, "zh-CN")
    : "zh-CN";
  const tag = normalizeLocaleTag(locale, fallbackTag);
  if (SUPPORTED_LANGUAGE_SET.has(tag)) return tag;
  const base = localeLanguageCode(tag, "zh");
  return BASE_LANGUAGE_TO_SUPPORTED_TAG[base] || fallbackTag;
}

export function isSupportedLocale(locale) {
  const tag = normalizeLocaleTag(locale, "");
  if (!tag) return false;
  if (SUPPORTED_LANGUAGE_SET.has(tag)) return true;
  return !!BASE_LANGUAGE_TO_SUPPORTED_TAG[localeLanguageCode(tag, "")];
}

export function localeDisplayName(locale, displayLocale = "zh-CN") {
  const tag = coerceSupportedLocale(locale);
  if (LABEL_OVERRIDES[tag]) return LABEL_OVERRIDES[tag];
  let localized = "";
  let native = "";
  try { localized = new Intl.DisplayNames([coerceSupportedLocale(displayLocale)], { type: "language" }).of(tag) || ""; } catch {}
  try { native = new Intl.DisplayNames([tag], { type: "language" }).of(tag) || ""; } catch {}
  const primary = localized || native || tag;
  return native && native !== primary ? `${primary} · ${native}` : primary;
}

export function buildLanguageOptions(displayLocale = "zh-CN") {
  const seen = new Set();
  const out = [];
  for (const raw of GLOBAL_LANGUAGE_TAGS) {
    const tag = coerceSupportedLocale(raw);
    if (seen.has(tag)) continue;
    seen.add(tag);
    out.push([tag, `${localeDisplayName(tag, displayLocale)} (${tag})`]);
  }
  return out;
}
