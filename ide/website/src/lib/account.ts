import { useEffect, useState } from "react";

/**
 * Who is signed in, as seen from the marketing site.
 *
 * The site and the account live on different hosts — mrday.one and code.mrday.one — so
 * this only works because the session cookie is scoped to `.mrday.one` and is readable by
 * script. The token is then sent as a normal `Authorization` header rather than as a
 * cookie, which keeps the gateway's `SameSite=Lax` intact: no cookie is sent cross-site,
 * so nothing about the site's existence weakens the console's CSRF posture.
 */
export const GATEWAY = "https://code.mrday.one";
export const DASHBOARD = `${GATEWAY}/dashboard`;

export type Account = {
  email: string;
  first_name?: string;
  last_name?: string;
  /** A `data:` image the account holder uploaded, or null. */
  avatar?: string | null;
};

/**
 * The session token, or "" when signed out.
 *
 * Exported because pages other than the account badge need to call the gateway as the
 * signed-in person — the rankings, for one, which is not shown to anonymous visitors.
 */
export function authToken(): string {
  try {
    const m = document.cookie.match(/(?:^|;\s*)mide_token=([^;]*)/);
    return m ? decodeURIComponent(m[1]) : "";
  } catch {
    return "";
  }
}

/** The name to greet someone by: their own name if they set one, otherwise the mailbox. */
export function displayName(a: Account): string {
  const full = [a.first_name, a.last_name].filter(Boolean).join(" ").trim();
  return full || a.email.split("@")[0] || a.email;
}

/**
 * The letter shown when the account has no picture.
 *
 * Deliberately identical to the console's rule — `(name || email || "?")` and its first
 * character — because the two are the same person's avatar seen in two places. This used
 * to take two initials on a grey circle while the console took one on a brand-coloured
 * one, so the same account appeared to have two different pictures depending on where you
 * looked. Nobody has uploaded an avatar yet, which means the placeholder *is* the avatar
 * for now and the mismatch was the whole of what people saw.
 */
export function avatarLetter(a: Account): string {
  const full = [a.first_name, a.last_name].filter(Boolean).join(" ").trim();
  return (full || a.email || "?").charAt(0).toUpperCase();
}

/**
 * `undefined` while unknown, `null` for signed out.
 *
 * The distinction matters: rendering "Log in" during the check and then swapping it for a
 * name is a flicker on every page load for everyone who is signed in. The caller shows
 * nothing until this resolves.
 */
export function useAccount(): Account | null | undefined {
  const [account, setAccount] = useState<Account | null | undefined>(undefined);

  useEffect(() => {
    const t = authToken();
    if (!t) {
      setAccount(null);
      return;
    }
    let alive = true;
    void (async () => {
      try {
        const res = await fetch(`${GATEWAY}/api/me`, {
          headers: { Authorization: `Bearer ${t}` },
          cache: "no-store",
        });
        // An expired or revoked token is a signed-out visitor, not an error worth showing.
        if (!res.ok) throw new Error(String(res.status));
        const body = (await res.json()) as Account;
        if (alive) setAccount(body.email ? body : null);
      } catch {
        if (alive) setAccount(null);
      }
    })();
    return () => {
      alive = false;
    };
  }, []);

  return account;
}
