import { useEffect, useState } from "react";
import { mseFetch } from "@/lib/mse";

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

/** How often to look at the cookie. Cheap — see `sync()`; this costs no network. */
const WATCH_MS = 3000;
/**
 * How stale a *verified* answer may get before the next tab focus re-checks it with the
 * server. Covers the one case the cookie cannot report: a session revoked from somewhere
 * else (the console's device list does exactly this) leaves the cookie sitting there
 * looking perfectly valid.
 */
const REVALIDATE_MS = 60_000;

/*
 * One watcher for the whole page, not one per component.
 *
 * The nav renders a badge for desktop and another for the mobile menu, and the rankings
 * read the session too — with the watching living inside the hook, every one of them ran
 * its own timer and fired its own /api/me on every change. Same question, same answer, N
 * requests. State module-level, subscribers get told: one request per actual change, no
 * matter how many things are asking.
 */
type Listener = (account: Account | null | undefined) => void;
const listeners = new Set<Listener>();

/** The answer on screen. `undefined` until the first one lands. */
let current: Account | null | undefined = undefined;
/** The token behind `current`, and when the server last confirmed it. */
let resolvedFor: string | null = null;
let verifiedAt = 0;
let timer = 0;

function publish(next: Account | null | undefined) {
  current = next;
  for (const listener of listeners) listener(next);
}

/** True when the answer is old enough that the server should be asked again. */
function stale(): boolean {
  return verifiedAt > 0 && Date.now() - verifiedAt > REVALIDATE_MS;
}

async function sync(force = false) {
  const token = authToken();
  // The heart of it: the cookie has not changed, so neither has the answer. This is what
  // makes the timer below free — it is a string comparison, not a request.
  if (!force && token === resolvedFor) return;
  resolvedFor = token;

  // No cookie is not an ambiguous state — it is signed out. Say so without asking.
  if (!token) {
    verifiedAt = 0;
    publish(null);
    return;
  }

  try {
    // Sealed, so the token stops travelling in a header any intermediary can read. The
    // client moves `Authorization` inside the envelope and strips it from the outer
    // request, which is the whole point of the exercise on the one call that carries a
    // session token on every tick.
    const res = await mseFetch(`${GATEWAY}/api/me`, {
      headers: { Authorization: `Bearer ${token}` },
      cache: "no-store",
    });
    // An expired or revoked token is a signed-out visitor, not an error worth showing.
    if (!res.ok) throw new Error(String(res.status));
    const body = (await res.json()) as Account;
    // The session can change while this request is in flight; a reply about a token nobody
    // is holding any more must not overwrite the current answer.
    if (authToken() !== token) return;
    verifiedAt = Date.now();
    publish(body.email ? body : null);
  } catch {
    if (authToken() !== token) return;
    publish(null);
    // Offline or a hiccup is not a settled answer. Forgetting it means the next tick tries
    // again, instead of showing "Log in" to a signed-in person until the cookie happens to
    // change on its own.
    resolvedFor = null;
    verifiedAt = 0;
  }
}

// Coming back to the tab is when the answer most often needs to have changed — signing in
// happened on another tab, and this is the moment it gets looked at again.
function onWake() {
  if (document.visibilityState === "hidden") return;
  void sync(stale());
}
function onStorage() {
  void sync();
}

function startWatching() {
  // pageshow as well as visibilitychange: returning via the Back button restores a page
  // frozen before the session changed, and that path fires nothing else here.
  document.addEventListener("visibilitychange", onWake);
  window.addEventListener("focus", onWake);
  window.addEventListener("pageshow", onWake);
  // Another tab of *this* origin signing out clears localStorage. Same-origin only, which
  // is why it is one signal among several rather than the mechanism.
  window.addEventListener("storage", onStorage);
  // Paused while hidden: a background tab has nobody to update, and onWake covers the
  // return. Cheap as it is, a timer that never sleeps is still a timer.
  timer = window.setInterval(() => {
    if (document.visibilityState === "visible") void sync();
  }, WATCH_MS);
}

function stopWatching() {
  window.clearInterval(timer);
  timer = 0;
  document.removeEventListener("visibilitychange", onWake);
  window.removeEventListener("focus", onWake);
  window.removeEventListener("pageshow", onWake);
  window.removeEventListener("storage", onStorage);
}

/**
 * `undefined` while unknown, `null` for signed out.
 *
 * The distinction matters: rendering "Log in" during the check and then swapping it for a
 * name is a flicker on every page load for everyone who is signed in. The caller shows
 * nothing until this resolves.
 *
 * **Kept live, not read once.** Signing in happens on code.mrday.one, not here — so with a
 * single check at mount this site would go on offering "Log in" to someone who signed in a
 * tab ago, until they thought to reload. The session lives in a cookie both hosts can see,
 * so the honest thing is to watch it. Signing out elsewhere lands instantly: an empty
 * cookie is conclusive on its own and needs no round trip to act on.
 */
export function useAccount(): Account | null | undefined {
  const [account, setAccount] = useState<Account | null | undefined>(current);

  useEffect(() => {
    listeners.add(setAccount);
    if (listeners.size === 1) startWatching();
    // Whatever is already known, immediately — a second badge mounting must not sit blank
    // waiting for a change that already happened.
    setAccount(current);
    void sync(stale());
    return () => {
      listeners.delete(setAccount);
      if (listeners.size === 0) stopWatching();
    };
  }, []);

  return account;
}
