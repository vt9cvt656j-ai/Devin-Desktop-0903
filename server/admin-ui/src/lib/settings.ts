/**
 * 运营参数的客户端镜像。唯一真相在服务端 `app_settings` / `plan_quotas` 表
 * （server/src/settings.rs），这里只是缓存一份用来渲染。
 *
 * 为什么需要这个文件：面值分母（663 真实计费分 = 客户看到的 $1.00）原先被硬编码了
 * 四份——Customers.tsx、Billing.tsx、static/admin.html、ide/src/main.js——其中三份在
 * **写路径**上：管理员输入的美元由前端乘 663 变成存库的真实分，服务端不做二次换算。
 * 也就是说，改服务端而不改前端，"发出去多少额度"会当场和"显示多少"对不上，而且没有
 * 任何报错。所以只要还有第二份字面量 663 存在，这个数就不能做成可配置的。
 *
 * 这里不引缓存库：一个运营台、一个操作员，参数一天改不了一次。模块级变量 +
 * useSyncExternalStore 足够，且与 api.ts "不引数据层依赖" 的取向一致。
 */
import { useSyncExternalStore } from "react";
import { api } from "./api";

export type PlanQuota = {
  plan: string;
  total_cents: number;
  window_cents: number;
  weekly_cents: number;
  days: number;
  rank: number;
  /** 这一行是否还等于代码里的出厂值 —— 用来把「没人设置过」和「你设的」区分开。 */
  is_default?: boolean;
};

export type AdminSettings = {
  raw_cents_per_credit_usd: number;
  free_points_daily: number;
  plans: PlanQuota[];
  limits: {
    raw_cents_per_credit_usd: [number, number];
    free_points_daily: [number, number];
  };
  raw_cents_per_point: number;
};

/** 网关答复之前的兜底，逐字等于改造前的硬编码值——首屏渲染行为不变。 */
export const FALLBACK: AdminSettings = {
  raw_cents_per_credit_usd: 663,
  free_points_daily: 40,
  plans: [],
  limits: { raw_cents_per_credit_usd: [1, 100000], free_points_daily: [0, 1000000] },
  raw_cents_per_point: 5,
};

let snapshot: AdminSettings = FALLBACK;
const listeners = new Set<() => void>();

function emit() {
  for (const l of listeners) l();
}

function subscribe(l: () => void) {
  listeners.add(l);
  return () => listeners.delete(l);
}

/**
 * 面值分母。**永远不会返回 0** —— 它在展示路径上是除数，0 会让每一个余额变成
 * Infinity。服务端有 CHECK 和 clamp，这里再兜一层，因为这是所有金额显示的入口。
 */
export function creditDenominator(): number {
  const n = Math.round(Number(snapshot.raw_cents_per_credit_usd));
  return Number.isFinite(n) && n >= 1 ? n : FALLBACK.raw_cents_per_credit_usd;
}

/** 真实计费分 → 面值分（客户看到的那个数，单位是分）。 */
export function creditCentsFromRaw(raw?: number | null): number {
  return Math.round(((raw || 0) / creditDenominator()) * 100);
}

/** 面值美元（输入框里的字符串）→ 存库的真实计费分。写路径。 */
export function rawCentsFromCreditDollars(dollars: string | number): number {
  const v = typeof dollars === "number" ? dollars : Number.parseFloat(dollars);
  return Math.round((Number.isFinite(v) ? v : 0) * creditDenominator());
}

export function currentSettings(): AdminSettings {
  return snapshot;
}

export function applySettings(next: Partial<AdminSettings>) {
  snapshot = { ...snapshot, ...next };
  emit();
}

let inflight: Promise<AdminSettings> | null = null;

/** 拉一次设置。重复调用共享同一个请求；失败保留兜底值，不让整台控制台白屏。 */
export async function loadSettings(force = false): Promise<AdminSettings> {
  if (inflight && !force) return inflight;
  inflight = api
    .get<AdminSettings>("/api/admin/settings")
    .then((s) => {
      snapshot = { ...FALLBACK, ...s };
      emit();
      return snapshot;
    })
    .catch(() => snapshot)
    .finally(() => {
      inflight = null;
    });
  return inflight;
}

/** 组件里用它订阅：设置到货后自动重渲染，金额不会停在兜底面值上。 */
export function useSettings(): AdminSettings {
  return useSyncExternalStore(subscribe, currentSettings, currentSettings);
}
