// Guard rails for the sign-in page's session handling.
//
// These exist because of one specific outage: signing out on the marketing site put the
// login page into an endless reload. Nobody could get in, and nobody could sign out.
//
// The shape of it is worth keeping in mind, because every assertion here is one half of
// the pair that has to stay in agreement:
//
//   nginx gates /dashboard on the `mide_token` COOKIE and nothing else.
//   gate.html decides "already signed in" from LOCALSTORAGE first, cookie second.
//
// So a token in localStorage without the matching cookie means the gate believes one
// thing and the route it redirects to believes the opposite — and the person ping-pongs
// between them forever. mrday.one and code.mrday.one are separate origins with separate
// localStorage, so signing out on the site could clear the shared cookie but never the
// gate's private copy. That is exactly the state that got produced.
//
// Scanning the source rather than running it: gate.html is one inline script inside a
// static page with no module boundary, and importing it would mean either a DOM harness
// or splitting the file — both bigger changes than the thing being protected.
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const HERE = dirname(fileURLToPath(import.meta.url));
const GATE = readFileSync(join(HERE, "..", "gate", "gate.html"), "utf8");

/** The body of a top-level `function name(` … up to the next top-level `\n  }`. */
function fnBody(src, name) {
  const at = src.indexOf(`function ${name}(`);
  assert.notEqual(at, -1, `gate.html no longer defines ${name}()`);
  const end = src.indexOf("\n  }", at);
  assert.notEqual(end, -1, `could not find the end of ${name}()`);
  return src.slice(at, end);
}

const RESUME = (() => {
  const at = GATE.indexOf("async function resume()");
  assert.notEqual(at, -1, "gate.html no longer has resume()");
  const end = GATE.indexOf("})();", at);
  assert.notEqual(end, -1, "could not find the end of resume()");
  return GATE.slice(at, end);
})();

test("every redirect into the gated app writes the session cookie first", () => {
  // The cookie is the only thing nginx checks. Redirecting without writing it sends the
  // person to a route that is certain to bounce them straight back here.
  const jumps = [...GATE.matchAll(/location\.replace\(nextTarget\(\)\)/g)];
  assert.ok(jumps.length > 0, "expected at least one redirect into the app");

  for (const jump of jumps) {
    // Look back a short way from each redirect for the cookie write. Short on purpose:
    // "somewhere earlier in the file" would pass even if the write sat in a branch that
    // never runs on this path.
    const before = GATE.slice(Math.max(0, jump.index - 400), jump.index);
    assert.match(
      before,
      /writeSessionCookie\(/,
      "a redirect into the app is not preceded by writeSessionCookie() — this is the " +
        "infinite-reload bug: nginx reads the cookie, and there is not one",
    );
  }
});

test("writeSessionCookie writes the shared-domain cookie, not a host-only one", () => {
  const body = fnBody(GATE, "writeSessionCookie");
  assert.match(body, /mide_token=/, "must write the cookie nginx reads");
  assert.match(
    body,
    /Domain=\.mrday\.one/,
    "host-only would be invisible to mrday.one, which is how the site ended up offering " +
      "'Log in' to someone who was plainly signed in",
  );
  assert.match(body, /Secure/, "session cookie must not travel in the clear");
  assert.match(body, /SameSite=Lax/, "Strict withholds it on the return from Stripe checkout");
});

test("resume() gives up after a redirect that comes straight back", () => {
  // The last line of defence. Everything else is meant to keep the gate and the guarded
  // route in agreement; if they ever disagree again, this turns an endless reload into a
  // login form, which is the difference between "broken" and "unusable".
  // The exact guard, not merely a mention of it: `if (false && cameStraightBack())` would
  // satisfy a looser check while leaving the loop wide open.
  assert.match(
    RESUME,
    /if \(cameStraightBack\(\)\) \{/,
    "resume() must detect a bounce; without it any future disagreement reloads forever",
  );
  const guard = RESUME.slice(RESUME.indexOf("cameStraightBack()"));
  assert.match(
    guard.slice(0, 600),
    /localStorage\.removeItem\("michael_token"\)/,
    "on a bounce the token this page was trusting must be dropped, or the next load " +
      "repeats the same redirect",
  );
  // And it has to happen before the network call, not after: the whole point is not to
  // act on that token again.
  assert.ok(
    RESUME.indexOf("cameStraightBack()") < RESUME.indexOf("/api/me"),
    "the bounce check must run before resume() asks the server about the token",
  );
});

test("the bounce marker is time-bounded", () => {
  const body = fnBody(GATE, "cameStraightBack");
  assert.match(
    body,
    /Date\.now\(\) - at < BOUNCE_WINDOW_MS/,
    "a flag with no clock would refuse to resume a good session for the rest of the tab's life",
  );
  const window = /var BOUNCE_WINDOW_MS = (\d+)/.exec(GATE);
  assert.ok(window, "BOUNCE_WINDOW_MS must be declared");
  const ms = Number(window[1]);
  // Long enough to cover a slow redirect, short enough that coming back on purpose
  // minutes later is not mistaken for a loop.
  assert.ok(ms >= 2000 && ms <= 30000, `bounce window ${ms}ms is outside the sensible range`);
});

test("a token the server rejects is dropped rather than retried on every visit", () => {
  const after = RESUME.slice(RESUME.indexOf("/api/me"));
  assert.match(
    after,
    /localStorage\.removeItem\("michael_token"\)/,
    "a revoked or expired token must be cleared once verified dead",
  );
});

test("sign-out revokes the session on the server before clearing anything local", () => {
  // The root cause, and the only fix that reaches across origins. mrday.one can delete the
  // shared cookie but cannot touch code.mrday.one's localStorage; revoking the session is
  // what makes that leftover copy worthless everywhere at once.
  const site = readFileSync(
    join(HERE, "..", "website", "src", "components", "site", "account-badge.tsx"),
    "utf8",
  );
  const signOut = site.slice(site.indexOf("async function signOut()"));
  const body = signOut.slice(0, signOut.indexOf("\n}\n"));
  assert.match(body, /\/api\/auth\/logout/, "the site's sign-out must revoke the session");
  assert.match(body, /method: "POST"/, "logout is a POST");
  assert.ok(
    body.indexOf("/api/auth/logout") < body.indexOf('localStorage.removeItem("michael_token")'),
    "the token has to still be readable when the revoke request is built",
  );
  // `mseFetch` 也算：MSE-1 上线后这条请求走加密替身，但它仍然是一个返回 Response 的
  // fetch，而这条断言守的是 **await**，不是函数名字。不加密的回退路径仍然是 fetch，
  // 所以两个名字都认。
  assert.match(
    body,
    /await (?:mse)?[fF]etch\(/,
    "not awaiting it lets the reload cancel the request — and that request is the fix",
  );
});
