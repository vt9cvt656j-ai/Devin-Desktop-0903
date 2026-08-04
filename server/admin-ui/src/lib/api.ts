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

export async function login(account: string, password: string) {
  const r = await api.post<{ token: string; user?: { role?: string } }>("/api/auth/login", {
    account,
    password,
  });
  if (!r?.token) throw new ApiError(500, "登录响应缺少 token");
  auth.set(r.token);
  // The old console checked the role client-side after login (admin.html:391). Keep that check —
  // it is what stops a paying customer from loading the operator shell.
  const me = await api.get<{ role?: string }>("/api/me");
  if (me?.role !== "admin") {
    auth.clear();
    throw new ApiError(403, "该账号不是管理员");
  }
  return me;
}
