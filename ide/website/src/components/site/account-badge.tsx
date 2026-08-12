import { useEffect, useRef, useState } from "react";
import { LayoutDashboard, LogIn, LogOut } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { DASHBOARD, GATEWAY, authToken, avatarLetter, displayName, useAccount } from "@/lib/account";

/**
 * "Log in", or who you already are.
 *
 * Someone arriving with a live session was being asked to log in again, which reads as
 * the site not knowing them. This shows their picture and name instead.
 *
 * Signed in, it is a menu rather than a link. The name used to be a plain link straight to
 * the dashboard, which left the site with **no way to sign out at all** — you had to go to
 * the console to do it, and the site would keep greeting you by name until you did.
 */

/**
 * End the session — on the server first, then everywhere it is written locally.
 *
 * The server call is not optional politeness. This site is on mrday.one and the account
 * lives on code.mrday.one; the two share the `.mrday.one` cookie but **not** localStorage,
 * and the sign-in page keeps its own copy of the token there. Clearing only what this
 * origin can reach left that copy alive and still valid, which produced a login page that
 * reloaded forever: the guarded route saw no cookie and bounced to the gate, the gate
 * found its leftover token, asked the server, was told it was fine, and sent the person
 * straight back. Revoking the session makes the leftover copy worthless, which is what
 * "sign out" was supposed to mean in the first place.
 *
 * Only this browser's session goes. The desktop app holds a different one and keeps it.
 *
 * Copied deliberately from the console's `signOut` rather than shared: the two are on
 * different hosts and different bundles, and the thing that must not drift is the *pair*
 * of cookie deletes. The cookie is written with `Domain=.mrday.one` so this site can see
 * it; a delete only matches a cookie with the same domain, so clearing the host-only form
 * alone leaves the wider one alive — and the site goes on greeting you by name after you
 * asked it not to.
 */
async function signOut() {
  const token = authToken();
  if (token) {
    try {
      // Awaited, so the session is revoked before the reload — a request still in flight
      // when the page navigates is a request the browser is free to drop, and dropping
      // this one is precisely the bug. keepalive covers the reload racing us anyway.
      await fetch(`${GATEWAY}/api/auth/logout`, {
        method: "POST",
        headers: { Authorization: `Bearer ${token}` },
        keepalive: true,
      });
    } catch {
      // Offline or the gateway is down. Sign out locally regardless: refusing to sign
      // someone out because the network is unavailable is the wrong way to fail.
    }
  }
  try {
    localStorage.removeItem("michael_token");
  } catch {
    /* nothing to clear */
  }
  document.cookie = "mide_token=; Domain=.mrday.one; Path=/; Max-Age=0";
  document.cookie = "mide_token=; Path=/; Max-Age=0";
  // Reload rather than flip local state: other parts of the page read the session too
  // (the rankings, for one), and a reload is the only way they all agree at once.
  location.reload();
}

export function AccountBadge({ className }: { className?: string }) {
  const account = useAccount();
  const [open, setOpen] = useState(false);
  const box = useRef<HTMLDivElement>(null);

  // Close on an outside click or on Escape. Both, because a menu that only closes by
  // clicking the trigger again is a menu you get stuck in.
  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (!box.current?.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDown);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  // Still checking. Deliberately renders nothing rather than "Log in": showing the
  // signed-out state first and correcting it a moment later is a flicker that every
  // signed-in visitor would see on every page load.
  if (account === undefined) {
    // h-9 matches the button that replaces it, so the nav does not shift when it resolves.
    return <div className={cn("h-9 w-24", className)} aria-hidden />;
  }

  if (account === null) {
    return (
      <Button variant="ghost" size="sm" asChild className={className}>
        <a href={`${GATEWAY}/gate`}>
          <LogIn /> Log in
        </a>
      </Button>
    );
  }

  const name = displayName(account);

  return (
    /*
      inline-block 而不是 block：桌面导航栏里这一块要收缩到内容宽度，不能撑满。
      移动端菜单传的是 w-full，那时它就撑满 —— 里面的按钮跟着 w-full，所以两种场景
      共用一套结构，不用给按钮和外壳分别传样式。
    */
    <div ref={box} className={cn("relative inline-block", className)}>
      <Button
        variant="ghost"
        size="sm"
        className="w-full justify-start"
        onClick={() => setOpen((v) => !v)}
        aria-haspopup="menu"
        aria-expanded={open}
        title={account.email}
      >
        {/*
          size-8 (32px) inside a 36px-tall button: it reads as an avatar rather than a
          bullet next to the name, and it is the size the console uses for the same
          person, so the two surfaces match at a glance.
        */}
        {account.avatar ? (
          <img src={account.avatar} alt="" className="size-8 shrink-0 rounded-full object-cover" />
        ) : (
          /* Same letter and colours as the console's avatar — see avatarLetter(). */
          <span
            aria-hidden
            className="flex size-8 shrink-0 items-center justify-center rounded-full bg-primary text-xs font-semibold text-primary-foreground"
          >
            {avatarLetter(account)}
          </span>
        )}
        {/* Long names would push the nav around, so the label is bounded and truncates. */}
        <span className="max-w-[10rem] truncate">{name}</span>
      </Button>

      {open && (
        <div
          role="menu"
          /*
            inset-x-0：外壳是 inline-block、宽度就是那颗按钮的宽度，左右都贴上去，
            菜单和「头像 + 名字」严丝合缝。
          */
          className="absolute inset-x-0 z-50 mt-1.5 overflow-hidden rounded-lg border border-border bg-background py-1 shadow-lg"
        >
          {/*
            一行身份，不是两行。
            以前这里名字和邮箱各占一行，而名字在上面那颗按钮里已经写着了 —— 同一个人被
            重复了一遍，还把菜单撑高一截；邮箱又刚好比按钮宽，只能截断成「…@qq.c...」，
            于是两行里没有一行是完整可读的。displayName 本来就是「有名字用名字，没有就用
            邮箱」，直接用它，一行就够。
          */}
          <div className="truncate px-3 py-1.5 text-xs text-muted-foreground">{name}</div>

          <a
            role="menuitem"
            href={DASHBOARD}
            className="flex items-center gap-2 px-3 py-1.5 text-[13px] transition-colors hover:bg-muted"
          >
            <LayoutDashboard className="size-3.5 shrink-0 text-muted-foreground" />
            进入后台
          </a>

          <button
            role="menuitem"
            type="button"
            onClick={() => void signOut()}
            className="flex w-full items-center gap-2 px-3 py-1.5 text-left text-[13px] transition-colors hover:bg-muted"
          >
            <LogOut className="size-3.5 shrink-0 text-muted-foreground" />
            退出登录
          </button>
        </div>
      )}
    </div>
  );
}
