import type { CatalogItem } from "@/lib/api";
import { DICTS, LOCALE_TAG, type Dict, type Lang } from "@/lib/i18n";

/**
 * Every `*_cents` field in this gateway is a "raw cent" whose value in dollars is set
 * server-side (`raw_cents_per_credit_usd`, currently 663). Never hardcode the divisor:
 * it is an operator setting, and three places used to disagree about it.
 */
let perUsd = 663;

export function setCreditDivisor(value: number | undefined): void {
  const n = Math.round(Number(value));
  if (Number.isFinite(n) && n >= 1 && n <= 100_000) perUsd = n;
}

/*
 * The locale every number on this page is formatted in.
 *
 * Module-level for the same reason `perUsd` is: money and counts are printed from dozens
 * of call sites, and threading a language argument through all of them to say one thing
 * would be noise. Set wherever the language is.
 */
let locale = "en-US";

export function setMoneyLocale(lang: Lang): void {
  locale = LOCALE_TAG[lang];
}

/** Dates follow the interface language, from the same table the selector uses. */
function localeFor(lang: Lang): string {
  return LOCALE_TAG[lang];
}

/**
 * Money, written the way the reader's language writes money.
 *
 * `Intl` rather than the hand-rolled symbol table this used to have. That table produced
 * "$19.99" in every language, but German writes 19,99 $ — symbol trailing, comma for the
 * decimal — and Spanish writes 19,99 US$. Intl also knows which currencies have no minor
 * unit, which the old `ZERO_DECIMAL` set was tracking by hand and would have got wrong
 * for every currency nobody remembered to add.
 *
 * `minimumFractionDigits: 0` keeps the old behaviour of dropping a trailing ".00", so a
 * plan at ¥4 still reads ¥4 rather than ¥4.00, while $19.99 prints in full.
 */
function money(value: number, currency: string, maxDigits = 2): string {
  return new Intl.NumberFormat(locale, {
    style: "currency",
    currency: currency.toUpperCase(),
    minimumFractionDigits: 0,
    maximumFractionDigits: maxDigits,
  }).format(value);
}

export function usd(rawCents: number | null | undefined, digits = 2): string {
  return money((Number(rawCents) || 0) / perUsd, "USD", digits);
}

/** A plain count: 1,500 in English, 1.500 in German, 1500 in Spanish. */
export function num(value: number): string {
  return new Intl.NumberFormat(locale).format(value);
}

/** The operator prices credits at 22 per US dollar. */
export function creditsOf(rawCents: number | null | undefined): number {
  return Math.round(((Number(rawCents) || 0) / perUsd) * 22);
}

/**
 * What the card prints, in the currency Stripe will actually take.
 *
 * The server decides which currency is honest to quote — USD only when the Stripe price
 * carries a USD amount, otherwise the price's own base currency — and sends the figure
 * already chosen. The page does no conversion of its own: inventing an exchange rate here
 * is how a card ends up advertising a number nobody is ever charged.
 */
export function price(item: CatalogItem, quantity = 1): string {
  const ccy = (item.display_currency || "usd").toUpperCase();
  const minor = item.display_minor;
  if (minor == null) return "—";
  // Intl carries each currency's own minor-unit exponent, so yen is not divided by 100.
  const digits = new Intl.NumberFormat(locale, { style: "currency", currency: ccy })
    .resolvedOptions().maximumFractionDigits;
  const major = (minor * quantity) / (digits === 0 ? 1 : 100);
  return money(major, ccy, digits);
}

/**
 * The currency symbol a product is quoted in, for the custom top-up's input prefix.
 *
 * Read out of a formatted value rather than kept as a second table, so it can never
 * disagree with the price beside it.
 */
export function currencySymbol(item: CatalogItem): string {
  const ccy = (item.display_currency || "usd").toUpperCase();
  const part = new Intl.NumberFormat(locale, { style: "currency", currency: ccy })
    .formatToParts(0)
    .find((p) => p.type === "currency");
  return part?.value ?? ccy;
}

export function formatDate(value: string | null | undefined, lang: Lang): string {
  if (!value) return "—";
  const d = new Date(value);
  if (Number.isNaN(d.getTime())) return "—";
  return d.toLocaleDateString(localeFor(lang), {
    year: "numeric",
    month: "long",
    day: "numeric",
  });
}

export function formatDateTime(value: string | null | undefined, lang: Lang): string {
  if (!value) return "—";
  const d = new Date(value);
  if (Number.isNaN(d.getTime())) return "—";
  return d.toLocaleString(localeFor(lang), {
    dateStyle: "medium",
    timeStyle: "short",
  });
}

/**
 * The included-quota window is 5h30m, so a bare date would be useless — say how long is
 * actually left.
 */
export function timeUntil(value: string | null | undefined): { text: string; expired: boolean } {
  if (!value) return { text: "", expired: false };
  const d = new Date(value);
  if (Number.isNaN(d.getTime())) return { text: "", expired: false };
  const ms = d.getTime() - Date.now();
  if (ms <= 0) return { text: "", expired: true };
  const mins = Math.round(ms / 60_000);
  const h = Math.floor(mins / 60);
  const m = mins % 60;
  return { text: h > 0 ? `${h}h ${m}m` : `${m}m`, expired: false };
}

/**
 * The plan's display name, translated.
 *
 * These used to be a hardcoded English table, which is why switching language left
 * "Ultra" and "Free" in English in the sidebar, in Settings and on every billing screen —
 * some of the most visible text in the console. The internal tier names (`ultra`,
 * `basic`) stay as they are: they are database values, not words for reading.
 */
const PLAN_KEYS: Record<string, keyof Dict> = {
  none: "planFree",
  trial: "planTrial",
  basic: "planBasic",
  pro: "planPro",
  power: "planPower",
  ultra: "planUltra",
};

export function planLabel(plan: string | null | undefined, lang: Lang): string {
  const t = DICTS[lang];
  if (!plan) return t.planFree;
  const key = PLAN_KEYS[plan];
  // An unknown tier is shown as-is rather than hidden: a plan nobody translated is
  // still a plan the account is on.
  return key ? t[key] : plan.charAt(0).toUpperCase() + plan.slice(1);
}

export function planIsActive(plan: string | null, expiresAt: string | null): boolean {
  if (!plan || plan === "none") return false;
  if (!expiresAt) return true;
  const d = new Date(expiresAt);
  return Number.isNaN(d.getTime()) ? true : d.getTime() > Date.now();
}
