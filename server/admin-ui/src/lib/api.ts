/**
 * The whole data layer. ~50 lines of fetch, deliberately no client library.
 *
 * The old console hand-rolled fetch 34 times with the token pasted in each call. This centralises
 * exactly three things — base URL, auth header, error shape — and nothing else. Adding TanStack
 * Query would be a real dependency for a console with one operator and no offline story; if
 * caching ever matters, it can be added later without touching call sites.
 */
const TOKEN_KEY = "michael_admin_token";

export const auth = {
  get: () => localStorage.getItem(TOKEN_KEY) || "",
  set: (t: string) => localStorage.setItem(TOKEN_KEY, t),
  clear: () => localStorage.removeItem(TOKEN_KEY),
};

export class ApiError extends Error {
  constructor(public status: number, message: string) {
    super(message);
  }
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(path, {
    ...init,
    headers: {
      "Content-Type": "application/json",
      ...(auth.get() ? { Authorization: `Bearer ${auth.get()}` } : {}),
      ...(init?.headers || {}),
    },
  });
  // 401 anywhere means the session is gone; surface it once rather than letting every screen
  // invent its own redirect.
  if (res.status === 401) {
    auth.clear();
    window.dispatchEvent(new CustomEvent("admin:unauthorized"));
    throw new ApiError(401, "登录已过期，请重新登录");
  }
  if (res.status === 204) return undefined as T;
  const body = await res.json().catch(() => ({}));
  if (!res.ok) {
    throw new ApiError(res.status, body?.error || body?.message || `请求失败 (${res.status})`);
  }
  return body as T;
}

export const api = {
  get: <T,>(p: string) => request<T>(p),
  post: <T,>(p: string, body?: unknown) =>
    request<T>(p, { method: "POST", body: JSON.stringify(body ?? {}) }),
  del: <T,>(p: string) => request<T>(p, { method: "DELETE" }),
};

/**
 * 服务端注销。/console/ 的门禁是一张 HttpOnly cookie，JS 删不掉它 —— 只有服务端能，
 * 而且服务端还要把 Redis 里的会话一起删掉，否则那张 cookie 到期前一直有效。
 * 这是这套系统第一次有真正的注销，而不是"前端把 token 丢了"。
 */
export async function endConsoleSession() {
  try {
    await fetch("/api/admin/session/logout", {
      method: "POST",
      credentials: "same-origin",
    });
  } catch {
    // 网络断了也要继续走本地清理和跳转：注销不能因为一个失败的请求就卡住。
  }
  auth.clear();
  window.location.replace("/console/login");
}

export async function login(identity: string, password: string) {
  // The WIRE field is still `email` — auth.rs LoginReq { email, password } — but the server now
  // resolves it as an email OR a username (find_user tries email first, then username). Keep
  // sending `email`; renaming the key would 422. Sending `account`, which is what the old console
  // labels the box, is exactly the bug that made login impossible earlier.
  const r = await api.post<{ token: string; user?: { role?: string; email?: string } }>(
    "/api/auth/login",
    { email: identity, password },
  );
  if (!r?.token) throw new ApiError(500, "登录响应缺少 token");
  auth.set(r.token);
  // The role is already on the login response (auth.rs returns { token, user }), so trust it
  // rather than paying a second round-trip. Keep the client-side check itself — it is what stops
  // a paying customer from loading the operator shell.
  const role = r.user?.role;
  if (role !== "admin") {
    auth.clear();
    throw new ApiError(403, "该账号不是管理员");
  }
  return r.user ?? {};
}
