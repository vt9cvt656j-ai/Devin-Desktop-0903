/**
 * Gateway client for the signed-in user's own account.
 *
 * nginx has already refused anyone without a valid session before this bundle is
 * served, so the token lookup here only finds the value to put in the Bearer header —
 * it is not the access control.
 */

export type Me = {
  id: string;
  email: string;
  role: string;
  /** US order — given name, then family name. "" when never set. */
  first_name: string;
  last_name: string;
  /** A `data:` URL the account holder uploaded, or null. */
  avatar: string | null;
  plan: string;
  plan_expires_at: string | null;
  credits_cents: number;
  free_points: number;
  free_points_daily: number;
  quota_total_cents: number;
  quota_window_cap_cents: number;
  quota_window_cents: number;
  quota_window_reset_at: string | null;
  quota_weekly_cap_cents: number;
  quota_week_used_cents: number;
  quota_week_reset_at: string | null;
  created_at: string;
  last_login_at: string | null;
  raw_cents_per_credit_usd?: number;
};

export type UsageRow = {
  cost_cents: number;
  prompt_tokens: number | null;
  completion_tokens: number | null;
  model: string;
  estimated: boolean;
  free_points_spent: number;
  time: string;
};

export type Usage = {
  credits_cents: number;
  plan: string;
  total_spent_cents: number;
  recent: UsageRow[];
};

export type CatalogItem = {
  lookup_key: string;
  label: string;
  kind: "plan" | "credits";
  plan: string | null;
  duration_days: number | null;
  credits_cents: number | null;
  amount_cents: number;
  amount_usd_cents: number | null;
  recurring: boolean;
  once_per_account: boolean;
  unit_credits_cents: number | null;
  blurb: string;
  already_purchased: boolean;
  included_cents: number | null;
  window_cap_cents: number | null;
  weekly_cap_cents: number | null;
};

export type Catalog = {
  enabled: boolean;
  raw_cents_per_credit_usd: number;
  /** Picked from the request's country by the gateway; only chooses the default tab. */
  currency: "cny" | "usd";
  country: string | null;
  items: CatalogItem[];
};

function token(): string {
  try {
    const t = localStorage.getItem("michael_token");
    if (t) return t;
  } catch {
    /* storage can be blocked; the cookie below is the fallback */
  }
  const m = document.cookie.match(/(?:^|;\s*)mide_token=([^;]*)/);
  return m ? decodeURIComponent(m[1]) : "";
}

export function toGate(): void {
  location.replace(`/gate?next=${encodeURIComponent(location.pathname)}`);
}

async function request<T>(path: string, init?: { method?: string; body?: unknown }): Promise<T> {
  const res = await fetch(path, {
    method: init?.method ?? "GET",
    headers: {
      Authorization: `Bearer ${token()}`,
      ...(init?.body ? { "Content-Type": "application/json" } : {}),
    },
    body: init?.body ? JSON.stringify(init.body) : undefined,
    cache: "no-store",
  });
  if (res.status === 401 || res.status === 403) {
    toGate();
    throw new Error("unauthorized");
  }
  const data = await res.json().catch(() => ({}));
  if (!res.ok) throw new Error((data as { error?: string }).error ?? `${path} → ${res.status}`);
  return data as T;
}

export const api = {
  hasToken: () => token() !== "",
  me: () => request<Me>("/api/me"),
  usage: () => request<Usage>("/api/usage"),
  catalog: () => request<Catalog>("/api/billing/catalog"),
  checkout: (lookup_key: string, quantity?: number) =>
    request<{ url: string }>("/api/billing/checkout", {
      method: "POST",
      body: quantity ? { lookup_key, quantity } : { lookup_key },
    }),
  /**
   * Partial update: only the keys present are written, so the picture can be changed
   * without resending the names. `avatar: ""` clears it; omitting it leaves it alone.
   */
  updateProfile: (body: { first_name?: string; last_name?: string; avatar?: string }) =>
    request<{ ok: boolean }>("/api/me/profile", { method: "POST", body }),
  redeem: (code: string) => request<unknown>("/api/redeem", { method: "POST", body: { code } }),
  models: () => request<unknown[]>("/api/models"),
};

export function signOut(): void {
  try {
    localStorage.removeItem("michael_token");
  } catch {
    /* nothing to clear */
  }
  document.cookie = "mide_token=; Path=/; Max-Age=0";
  location.replace("/gate");
}

/**
 * The desktop app answers on loopback when it is running and signed in. Probed in
 * order rather than all at once: a refused connection is logged by the browser's
 * network stack and cannot be caught away, so the common case should make one request.
 */
const HANDOFF_PORTS = [47821, 47822, 47823];

export type DesktopSession = { app: string; version: string; signedIn: boolean; email: string | null };

export async function probeDesktop(index = 0): Promise<DesktopSession | null> {
  if (index >= HANDOFF_PORTS.length) return null;
  const ctl = new AbortController();
  const timer = setTimeout(() => ctl.abort(), 700);
  try {
    const res = await fetch(`http://127.0.0.1:${HANDOFF_PORTS[index]}/session`, {
      headers: { "X-MrDay-Handoff": "1" },
      mode: "cors",
      cache: "no-store",
      signal: ctl.signal,
    });
    if (!res.ok) throw new Error("bad status");
    const json = (await res.json()) as DesktopSession;
    if (json.app !== "mrday-one") throw new Error("not ours");
    return json;
  } catch {
    return probeDesktop(index + 1);
  } finally {
    clearTimeout(timer);
  }
}
