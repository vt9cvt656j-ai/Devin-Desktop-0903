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
  integrations: () => request<{ providers: Integration[] }>("/api/integrations"),
  /** Returns the provider's authorize URL; the caller navigates to it. */
  integrationStart: (provider: string) =>
    request<{ url: string }>(`/api/integrations/${provider}/start`),
  /** Verified against the provider before it is stored, so a bad paste fails here. */
  integrationConnectToken: (provider: string, token: string) =>
    request<{ ok: boolean; account_login: string }>(`/api/integrations/${provider}/token`, {
      method: "POST",
      body: { token },
    }),
  integrationDisconnect: (provider: string) =>
    request<{ ok: boolean; revoke_at_provider: string }>(`/api/integrations/${provider}`, {
      method: "DELETE",
    }),
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

export type Integration = {
  provider: string;
  label: string;
  /**
   * Whether the one-click OAuth button can be offered. Linking by pasted token needs
   * nothing registered on the server, so it stays available either way — this only
   * decides whether there is a second, faster route.
   */
  oauth_configured: boolean;
  /** Where the person creates a personal access token by hand. */
  token_url: string;
  /** Which scopes to tick there — the usual reason a pasted token gets refused. */
  token_hint: string;
  connected: boolean;
  account_login: string | null;
  account_name: string | null;
  avatar_url: string | null;
  connected_at: string | null;
};

export type DesktopSession = { app: string; version: string; signedIn: boolean; email: string | null };

/**
 * Why a reachable, correctly-answering app can still look absent.
 *
 * Chrome 150 replaced Private Network Access — where a loopback server opted in by
 * answering the preflight with `Access-Control-Allow-Private-Network: true` — with a
 * *permission* the person has to grant. The desktop app still sends that header and it
 * is no longer enough on its own: until the permission is granted, the fetch fails with
 * a bare "Failed to fetch", indistinguishable from the app not running.
 *
 * Chrome only shows the prompt off a user gesture, so a probe on page load can never
 * obtain it — it just fails, silently, forever. Hence the distinct states: the page has
 * to be able to say "your browser is holding this back, click here" instead of the flat
 * "not running" it used to claim while the app was running perfectly.
 */
export type DesktopProbe =
  | { state: "connected"; session: DesktopSession }
  /**
   * Could not reach it. Deliberately NOT called "not running" any more: a refused
   * connection and a browser-blocked one are the same `TypeError: Failed to fetch`, so
   * claiming the app is absent was a guess the page kept getting wrong while the app was
   * running perfectly. `permission` and `error` carry what was actually observed so the
   * card can show it instead of inventing a cause.
   */
  | { state: "unreachable"; permission: string; error: string }
  /** Reachable in principle; Chrome needs a click to raise the permission prompt. */
  | { state: "needs-permission" }
  /** The person said no, or a policy did. Only site settings can undo it. */
  | { state: "permission-blocked" };

/** null when the browser has no such permission — then the old rules still apply. */
async function localNetworkPermission(): Promise<PermissionState | null> {
  try {
    const status = await navigator.permissions.query({
      name: "local-network-access" as PermissionName,
    });
    return status.state;
  } catch {
    return null;
  }
}

/**
 * One probe pass. Call it from a click to let Chrome raise the permission prompt; call
 * it on load to report the current state without one.
 */
export async function probeDesktop(): Promise<DesktopProbe> {
  const attempt = await reachDesktop();
  if (attempt.session) return { state: "connected", session: attempt.session };

  const permission = await localNetworkPermission();
  if (permission === "denied") return { state: "permission-blocked" };
  if (permission === "prompt") return { state: "needs-permission" };

  // Permission is "granted", or the browser has no such permission at all — and it still
  // could not be reached. There is no way from here to tell a stopped app from a browser
  // that is blocking for some other reason, so the page reports what was seen rather
  // than picking one and stating it as fact.
  return {
    state: "unreachable",
    permission: permission ?? "unsupported",
    error: attempt.error,
  };
}

/** Carries the last failure back up, so the card can show it instead of a guess. */
async function reachDesktop(
  index = 0,
  lastError = "no attempt made",
): Promise<{ session: DesktopSession | null; error: string }> {
  if (index >= HANDOFF_PORTS.length) return { session: null, error: lastError };
  const ctl = new AbortController();
  const timer = setTimeout(() => ctl.abort(), 700);
  try {
    const res = await fetch(`http://127.0.0.1:${HANDOFF_PORTS[index]}/session`, {
      headers: { "X-MrDay-Handoff": "1" },
      mode: "cors",
      cache: "no-store",
      signal: ctl.signal,
    });
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    const json = (await res.json()) as DesktopSession;
    if (json.app !== "mrday-one") throw new Error("something else is on that port");
    return { session: json, error: "" };
  } catch (e) {
    const why = e instanceof Error ? `${e.name}: ${e.message}` : String(e);
    return reachDesktop(index + 1, `port ${HANDOFF_PORTS[index]} — ${why}`);
  } finally {
    clearTimeout(timer);
  }
}
