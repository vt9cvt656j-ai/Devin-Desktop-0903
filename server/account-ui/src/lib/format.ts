import type { CatalogItem } from "@/lib/api";
import type { Currency, Lang } from "@/lib/i18n";

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

export function usd(rawCents: number | null | undefined, digits = 2): string {
  return `$${((Number(rawCents) || 0) / perUsd).toFixed(digits)}`;
}

/** The operator prices credits at 22 per US dollar. */
export function creditsOf(rawCents: number | null | undefined): number {
  return Math.round(((Number(rawCents) || 0) / perUsd) * 22);
}

const trimZeros = (s: string) => s.replace(/\.00$/, "");

/**
 * A product carries two independent prices, never one converted into the other: the
 * operator sets each dollar figure by hand, so ¥88 is $12.99 because they said so.
 */
export function price(item: CatalogItem, currency: Currency, quantity = 1): string {
  if (currency === "usd") {
    return item.amount_usd_cents == null
      ? "—"
      : `$${trimZeros(((item.amount_usd_cents * quantity) / 100).toFixed(2))}`;
  }
  return `¥${trimZeros(((item.amount_cents * quantity) / 100).toFixed(2))}`;
}

export function priceAlt(item: CatalogItem, currency: Currency, quantity = 1): string {
  if (currency === "usd") return `¥${trimZeros(((item.amount_cents * quantity) / 100).toFixed(2))}`;
  return item.amount_usd_cents == null
    ? ""
    : `$${trimZeros(((item.amount_usd_cents * quantity) / 100).toFixed(2))}`;
}

export function formatDate(value: string | null | undefined, lang: Lang): string {
  if (!value) return "—";
  const d = new Date(value);
  if (Number.isNaN(d.getTime())) return "—";
  return d.toLocaleDateString(lang === "zh" ? "zh-CN" : undefined, {
    year: "numeric",
    month: "long",
    day: "numeric",
  });
}

export function formatDateTime(value: string | null | undefined, lang: Lang): string {
  if (!value) return "—";
  const d = new Date(value);
  if (Number.isNaN(d.getTime())) return "—";
  return d.toLocaleString(lang === "zh" ? "zh-CN" : undefined, {
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

export const PLAN_NAMES: Record<string, string> = {
  none: "Free",
  trial: "Trial",
  basic: "Basic",
  pro: "Pro",
  power: "Power",
  ultra: "Ultra",
};

export function planLabel(plan: string | null | undefined): string {
  if (!plan) return "Free";
  return PLAN_NAMES[plan] ?? plan.charAt(0).toUpperCase() + plan.slice(1);
}

export function planIsActive(plan: string | null, expiresAt: string | null): boolean {
  if (!plan || plan === "none") return false;
  if (!expiresAt) return true;
  const d = new Date(expiresAt);
  return Number.isNaN(d.getTime()) ? true : d.getTime() > Date.now();
}
