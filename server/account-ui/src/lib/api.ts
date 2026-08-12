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
  /** BCP-47 interface language, or "" when never chosen. */
  language: string;
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
  /**
   * Which currency the card should print, and the figure in that currency's minor units.
   * Both are decided server-side from what Stripe will actually charge, so the page never
   * picks between two amounts or applies a rate of its own.
   */
  display_currency: string;
  display_minor: number | null;
  recurring: boolean;
  once_per_account: boolean;
  unit_credits_cents: number | null;
  blurb: string;
  /** Per-language product text from Stripe metadata, keyed by BCP-47 tag. */
  labels: Record<string, string>;
  blurbs: Record<string, string>;
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

/**
 * Re-issue this session's cookie on `.mrday.one`.
 *
 * Sessions created before the cookie was widened are host-only, so the marketing site
 * cannot see them and keeps offering "Log in" to someone who is plainly signed in. Only
 * the sign-in page writes the cookie, and a signed-in person has no reason to go back
 * there — so without this the old sessions would stay invisible until they expired.
 *
 * Writing it here fixes them on the next visit to the console. Harmless when the cookie
 * is already correct: same name, same value, same scope.
 */
export function ensureSharedSession(): void {
  const t = token();
  if (!t) return;
  try {
    document.cookie =
      `mide_token=${encodeURIComponent(t)}; Domain=.mrday.one; Path=/; Secure; SameSite=Lax; Max-Age=${7 * 24 * 3600}`;
  } catch {
    /* nothing depends on this succeeding */
  }
}

export function toGate(): void {
  location.replace(`/gate?next=${encodeURIComponent(location.pathname)}`);
}

export type Referral = {
  granted: boolean;
  /** true = commission lands as account credit; there is then nothing to withdraw. */
  auto_settle: boolean;
  /** true = 系统按冻结期和门槛自动打款，用户不需要（也不能）自己发起提现。 */
  batch_enabled: boolean;
  /** This account's outstanding requests. Keeps the screen reachable across a switch. */
  pending_withdrawals: number;
  rate_bps: number;
  window_days: number;
  /** The programme-wide switch. Off means nobody can bind new referrals right now. */
  enabled: boolean;
  code?: string;
  link?: string;
  invited?: number;
  pending_cents?: number;
  settled_cents?: number;
};

export type MyReferral = {
  /** Masked by the gateway, e.g. `h***0@gmail.com`. */
  who: string;
  source: string;
  rate_bps: number;
  created_at: string;
  expires_at: string;
  active: boolean;
  earned_cents: number;
};

export type MySettlement = {
  id: string;
  /** Masked by the gateway. */
  customer_email: string;
  amount_cents: number;
  rate_bps: number;
  commission_cents: number;
  /** 'auto' = credited to your balance; 'manual' = an operator approved it. */
  settled_by: "auto" | "manual";
  settled_at: string | null;
  created_at: string;
  /** 'settled', or 'reversed' when the purchase behind it was refunded. */
  status: string;
  /** Set when that purchase was refunded or charged back. */
  reversed_at: string | null;
  reversal_reason: string;
};

export type SettlementList = {
  rows: MySettlement[];
  page: number;
  pages: number;
  total: number;
  total_cents: number;
  per_page: number;
};

export type Withdrawal = {
  id: string;
  amount_cents: number;
  method: string;
  account: string;
  /** pending | paid | rejected | failed | returned */
  status: string;
  note: string;
  created_at: string;
  paid_at: string | null;
  /** 'manual'(有人手动转的) | 'stripe_connect'(系统自动转的) */
  provider?: string;
  /** 自动打款失败或被冲回的原因，Stripe 的原话。 */
  failure_reason?: string;
};

/** 一次支付的结果。见 stripe.rs session_result。 */
export type PaymentResult = {
  paid: boolean;
  kind: string;
  label: string | null;
  plan: string | null;
  duration_days: number | null;
  /** 这一单买到的钱包额度（原始计费分），套餐单为 null。 */
  credits_cents: number | null;
  amount_cents: number;
  /** Stripe 实收，以及币种。 */
  charged_cents: number | null;
  charged_currency: string | null;
  raw_cents_per_credit_usd: number;
  /** 发放之后账号的实际状态 —— 从账号读，不是复述订单。 */
  account: {
    plan: string;
    plan_expires_at: string | null;
    credits_cents: number;
    quota_total_cents: number;
  };
};

/** 自动打款的开通状态。见 connect.rs。 */
export type ConnectState = {
  /** 平台侧根本没配 Stripe 时为 false —— 这时不该显示开通按钮。 */
  configured: boolean;
  connected: boolean;
  ready: boolean;
  missing: string[];
};

export type WithdrawState = {
  available_cents: number;
  /** Earned but not yet approved, so not yet withdrawable. Shown so the gap is explained. */
  pending_commission_cents: number;
  min_cents: number;
  methods: string[];
  rows: Withdrawal[];
};

/** Where the gate parks a `?ref=` code until there is a session to attach it to. */
export const REF_KEY = "mide_ref";

/**
 * Attach a pending referral, once, after signing in.
 *
 * Runs here rather than in the gate because the gate is not on every path in: signing in
 * with GitHub or Google redirects from the provider straight to /dashboard with a cookie,
 * never passing through the gate's own completion code. The console is the one place every
 * route arrives at, so it is the only place this works for all of them.
 *
 * The key is cleared on success and on a refusal (already referred, own code, unknown
 * code) — those are answers, and retrying them forever would be pointless. It is kept on a
 * network error, which is not an answer and may work on the next load.
 */
export async function claimPendingReferral(): Promise<ReferralOutcome> {
  // localStorage 优先，cookie 兜底：隐私模式下 gate 写 localStorage 会静默失败，
  // 那时候唯一还留着邀请码的就是 cookie（gate 两条路都镜像了一份）。
  let code = "";
  try {
    code = localStorage.getItem(REF_KEY) ?? "";
  } catch {
    /* 存储被禁，往下走 cookie */
  }
  if (!code) {
    const hit = document.cookie.match(/(?:^|;\s*)mide_ref=([^;]+)/);
    code = hit ? decodeURIComponent(hit[1]) : "";
  }
  if (!code) return { kind: "none" };

  try {
    await api.claimReferral(code);
  } catch (e) {
    /*
     * 只有服务端**明确拒绝**这个码时才丢掉它。
     *
     * 以前这里只把断网（TypeError）当作可重试，于是任何 HTTP 错误都会走到下面的
     * removeItem —— 一次 500（数据库抖一下）、一次 502（网关正在重启）、一次 401
     * （令牌过期），都会把这个码唯一的副本永久删掉。码只在 gate 里写过一次，
     * 没有第二份，删了就再也绑不上，而且没有任何人会收到提示。
     *
     * 400 才是「这个码不能用」：无效、已绑过、是自己的、账号已有付款记录。那些情况
     * 留着也没意义。其余一律保留 —— 控制台每次挂载都会重跑这个函数，留着就等于自动重试。
     */
    const status = e instanceof ApiError ? e.status : 0;
    const refused = status === 400;
    // 没送到就留着下次再试；被明确拒绝才丢掉，并且把服务端的原话带出去 ——
    // 用户点了一个邀请链接却什么都没发生，是这条链上最让人困惑的一步。
    if (!refused) return { kind: "retry" };
    clearRef();
    return { kind: "refused", message: e instanceof Error ? e.message : "" };
  }
  clearRef();
  return { kind: "bound" };
}

/** 本地两份邀请码一起清 —— 只清一份，下次挂载会拿另一份重试一个已经处理过的码。 */
function clearRef() {
  try {
    localStorage.removeItem(REF_KEY);
  } catch {
    /* private mode — nothing to clear */
  }
  document.cookie = "mide_ref=; Path=/; Max-Age=0; SameSite=Lax; Secure";
}

/** 这次绑定的结果。`retry` 表示请求没送到，码还留着，下次挂载会自动再试。 */
export type ReferralOutcome =
  | { kind: "none" }
  | { kind: "bound" }
  | { kind: "retry" }
  | { kind: "refused"; message: string };

/**
 * 系统语言、时区和 UTC 偏移。
 *
 * 服务端拿这三个加上 IP 判定是不是中国区（见 stripe.rs buyer_currency）。时区名和偏移
 * 一起报，是为了让服务端能校验这两者自洽 —— 只报一个时区名的话，随手填一个就过了。
 *
 * 全部包在 try 里：老浏览器上 Intl 可能缺失，缺了就少报一个信号，不能让整个请求挂掉。
 */
function regionSignals(): Record<string, string> {
  try {
    return {
      "x-ide-language": navigator.language || "",
      "x-ide-timezone": Intl.DateTimeFormat().resolvedOptions().timeZone || "",
      "x-ide-utc-offset-minutes": String(-new Date().getTimezoneOffset()),
    };
  } catch {
    return {};
  }
}

/**
 * 带状态码的错误。
 *
 * 以前所有失败都抛同一个 `Error`，调用方只能靠字符串猜发生了什么 —— 而「服务器拒绝了」和
 * 「请求根本没送到」在邀请码这条路上的后果完全相反：前者该丢掉码，后者必须留着重试。
 */
export class ApiError extends Error {
  constructor(
    message: string,
    readonly status: number,
  ) {
    super(message);
    this.name = "ApiError";
  }
}

async function request<T>(path: string, init?: { method?: string; body?: unknown }): Promise<T> {
  const res = await fetch(path, {
    method: init?.method ?? "GET",
    headers: {
      Authorization: `Bearer ${token()}`,
      ...(init?.body ? { "Content-Type": "application/json" } : {}),
      // 中国区判定的三个信号。必须挂在**每一个**请求上：定价卡片和结账是两次独立的请求，
      // 只给其中一个加，两边算出来的币种就可能不一致 —— 那是一笔拒付，不是显示问题。
      //
      // 偏移量的符号要翻过来：getTimezoneOffset() 对东八区返回 -480，而服务端比对的是
      // 「UTC+480」这个方向。
      ...regionSignals(),
    },
    body: init?.body ? JSON.stringify(init.body) : undefined,
    cache: "no-store",
  });
  if (res.status === 401 || res.status === 403) {
    toGate();
    throw new ApiError("unauthorized", res.status);
  }
  const data = await res.json().catch(() => ({}));
  if (!res.ok) {
    throw new ApiError((data as { error?: string }).error ?? `${path} → ${res.status}`, res.status);
  }
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
  updateProfile: (body: { first_name?: string; last_name?: string; avatar?: string; language?: string }) =>
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
  /**
   * This account's referral standing.
   *
   * `granted: false` is a normal answer, not an error — referring is a privilege an admin
   * hands out, so most accounts get that and the page renders "ask an admin" rather than
   * a failure.
   */
  referral: () => request<Referral>("/api/referral/me"),
  /** The people this account brought in. Addresses are masked by the gateway. */
  myReferrals: () => request<MyReferral[]>("/api/referral/referrals"),
  mySettlements: (page: number) =>
    request<SettlementList>(`/api/referral/settlements?page=${page}`),
  /** Balance, limits and past requests. Nothing here moves money — see Withdraw.tsx. */
  withdrawals: () => request<WithdrawState>("/api/referral/withdrawals"),
  requestWithdrawal: (body: { amount_cents: number; method: string; account: string; qr?: string }) =>
    request<{
      id: string;
      available_cents: number;
      /** true = 已经直接打款到对方 Stripe 账户，没有人工环节。 */
      auto_paid: boolean;
      transfer_id: string | null;
    }>("/api/referral/withdraw", { method: "POST", body }),
  /** 这一笔支付买到了什么。订单没到账时后端会主动向 Stripe 核实并当场发放。 */
  paymentResult: (sessionId: string) =>
    request<PaymentResult>(`/api/billing/session/${encodeURIComponent(sessionId)}`),
  /** 这个账号能不能自动收款，缺什么。 */
  connect: () => request<ConnectState>("/api/referral/connect"),
  /** 拿一条 Stripe 开户链接。链接是一次性的，每次点都重新要。 */
  connectStart: () => request<{ url: string }>("/api/referral/connect/start", { method: "POST" }),
  /** Bind whoever owns `code` as this account's referrer. See `claimPendingReferral`. */
  claimReferral: (code: string) =>
    request<{ ok: boolean }>("/api/referral/claim", { method: "POST", body: { code } }),
  redeem: (code: string) => request<unknown>("/api/redeem", { method: "POST", body: { code } }),
  models: () => request<unknown[]>("/api/models"),
  sessions: () => request<SessionList>("/api/sessions"),
  revokeSession: (id: string) =>
    request<{ ok: boolean }>(`/api/sessions/${id}`, { method: "DELETE" }),
};

/**
 * 退出登录。先在服务端作废这一次登录，再清本地。
 *
 * 服务端那一步不是可有可无的。令牌的副本不止一处：这里的 localStorage、`.mrday.one` 那颗
 * 共享 cookie、官网所在源的 localStorage —— 谁也删不掉别的源里的那份。只清本地，等于把
 * 一张仍然有效的令牌留在别处；登录页读到它、问服务端、被告知有效，就会把人送回受门禁的
 * 页面，而那里只认 cookie，于是又弹回登录页 —— 无限刷新。作废之后，残留副本在哪儿都换
 * 不到东西。
 *
 * 只作废当前浏览器这一条，桌面端的登录不受影响 —— 界面上那句提示说的就是这个意思。
 */
export async function signOut(): Promise<void> {
  const t = token();
  if (t) {
    try {
      // 等它回来再跳转：请求还在飞的时候导航，浏览器可以直接把它丢掉，而丢掉的正好
      // 就是让退出登录生效的那一步。keepalive 再兜一层。
      await fetch("/api/auth/logout", {
        method: "POST",
        headers: { Authorization: `Bearer ${t}` },
        keepalive: true,
      });
    } catch {
      // 断网或网关挂了。照样退出：因为网络不通就拒绝让人退出登录，是错误的失败方式。
    }
  }
  try {
    localStorage.removeItem("michael_token");
  } catch {
    /* nothing to clear */
  }
  // Cleared twice on purpose. The cookie is written with Domain=.mrday.one so the
  // marketing site can see the session, and a delete only matches a cookie with the same
  // domain — clearing the host-only form alone would leave the wider one alive, and
  // signing out here would leave the site still greeting you by name.
  document.cookie = "mide_token=; Domain=.mrday.one; Path=/; Max-Age=0";
  document.cookie = "mide_token=; Path=/; Max-Age=0";
  location.replace("/gate");
}

/**
 * The desktop app answers on loopback when it is running and signed in. Probed in
 * order rather than all at once: a refused connection is logged by the browser's
 * network stack and cannot be caught away, so the common case should make one request.
 */
const HANDOFF_PORTS = [47821, 47822, 47823];

export type DeviceSession = {
  id: string;
  /** 'web' | 'desktop' | 'mobile' — picks the icon. */
  kind: string;
  /** Server-composed English. Kept for reference; the console builds its own. */
  label: string;
  /** Proper nouns from the User-Agent, or null. Never translated — "Chrome" is Chrome. */
  browser: string | null;
  platform: string | null;
  ip: string;
  created_at: string;
  last_seen_at: string;
  /** The session this page is being read from. */
  current: boolean;
};

export type SessionList = {
  sessions: DeviceSession[];
  /**
   * False when this browser's token predates session tracking. Such a token still works
   * but is not in the list and cannot be revoked on its own, so the page says so rather
   * than presenting a list that looks complete.
   */
  current_tracked: boolean;
};

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

export type DesktopSession = {
  app: string;
  version: string;
  signedIn: boolean;
  email: string | null;
  /** True when the gateway answered, rather than the app over localhost. */
  viaServer?: boolean;
  /** How stale the gateway's last heartbeat is, in seconds. */
  secondsAgo?: number;
};

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
export type DesktopStatus = {
  online: boolean;
  version: string | null;
  seconds_ago: number | null;
  interval_secs: number;
};

export async function probeDesktop(): Promise<DesktopProbe> {
  // Ask the gateway first. The app reports in on a timer, so this needs no loopback, no
  // browser permission, and works with the console open on another machine. The
  // localhost probe below is kept only because, when it does work, it proves the app is
  // on *this* computer — a stronger claim than the server can make.
  try {
    const s = await request<DesktopStatus>("/api/desktop/status");
    if (s.online) {
      return {
        state: "connected",
        session: {
          app: "mrday-one",
          version: s.version ?? "",
          signedIn: true,
          email: null,
          viaServer: true,
          secondsAgo: s.seconds_ago ?? 0,
        },
      };
    }
  } catch {
    // Older gateway without the endpoint, or offline. Fall through to the local probe.
  }

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
