import { useState } from "react";
import { Menu, X } from "lucide-react";
import { AccountBadge } from "@/components/site/account-badge";
import { Button } from "@/components/ui/button";
import { ThemeToggle } from "@/components/theme-toggle";
import { cn } from "@/lib/utils";

const CONSOLE = "https://code.mrday.one";

/*
 * Destinations, not in-page anchors.
 *
 * These used to scroll to sections of this page. They now navigate, which is why they no
 * longer start with "#" — the delegated click handler in main.tsx only intercepts hash
 * links, so these leave the page the way an ordinary link should.
 *
 * `soon` is for a destination that does not exist yet. A nav button pointing at a page
 * nobody has built is a 404 with the site's own branding on it, which is worse than
 * saying plainly that it is not ready — the same convention the sign-in page already
 * uses for the providers it cannot offer.
 *
 * Model is an account page: a signed-out visitor following it lands on the sign-in page and
 * is returned afterwards, which is the gateway's normal behaviour. Update Log is public.
 * Rankings is a page of this site but not a public one — it reports what accounts spent, so
 * it renders its own "sign in to see this" rather than redirecting; the nav should not
 * bounce someone off the site for clicking a tab.
 */
const links: { href?: string; label: string; soon?: boolean }[] = [
  { href: "/", label: "Home" },
  { href: `${CONSOLE}/dashboard#models`, label: "Model" },
  { href: "/changelog", label: "Update Log" },
  { href: "/rankings", label: "Rankings" },
];

const linkClass =
  "rounded-lg px-3 py-2 text-sm font-medium text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground";

export function Navbar() {
  const [open, setOpen] = useState(false);

  return (
    <header className="sticky top-0 z-50 border-b border-border/60 bg-background/80 backdrop-blur-xl">
      <nav className="mx-auto flex h-16 max-w-6xl items-center justify-between px-4 sm:px-6">
        {/* 标题栏式左组：窗口控制点 → 日/夜切换 → 品牌 */}
        <div className="flex items-center gap-3">
          <ThemeToggle />
          <a href="#" className="flex items-center gap-2.5 font-display text-lg font-semibold">
            <img src="/logo.png" alt="" className="size-8" />
            Mr. Day One
          </a>
        </div>

        <div className="hidden items-center gap-1 md:flex">
          {links.map((link) =>
            link.soon ? (
              <span
                key={link.label}
                className={cn(linkClass, "flex cursor-not-allowed items-center gap-1.5 opacity-60 hover:bg-transparent hover:text-muted-foreground")}
                title="Not published yet"
              >
                {link.label}
                <span className="rounded-full border border-border px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-[0.04em]">
                  Soon
                </span>
              </span>
            ) : (
              <a key={link.label} href={link.href} className={linkClass}>
                {link.label}
              </a>
            ),
          )}
        </div>

        <div className="hidden items-center gap-2 md:flex">
          <AccountBadge />
          <Button size="sm" asChild>
            <a href="#download">Download</a>
          </Button>
        </div>

        <div className="flex items-center gap-1 md:hidden">
          <Button
            variant="ghost"
            size="icon"
            aria-label={open ? "Close menu" : "Open menu"}
            onClick={() => setOpen((v) => !v)}
          >
            {open ? <X /> : <Menu />}
          </Button>
        </div>
      </nav>

      <div className={cn("border-t border-border/60 px-4 pb-4 md:hidden", open ? "block" : "hidden")}>
        {links.map((link) =>
          link.soon ? (
            <span
              key={link.label}
              className="flex items-center gap-1.5 rounded-lg px-3 py-2.5 text-sm font-medium text-muted-foreground opacity-60"
            >
              {link.label}
              <span className="rounded-full border border-border px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-[0.04em]">
                Soon
              </span>
            </span>
          ) : (
            <a
              key={link.label}
              href={link.href}
              onClick={() => setOpen(false)}
              className="block rounded-lg px-3 py-2.5 text-sm font-medium text-muted-foreground hover:bg-secondary hover:text-foreground"
            >
              {link.label}
            </a>
          ),
        )}
        {/* Same account state as the desktop bar, so the two never disagree. */}
        <AccountBadge className="mt-2 w-full justify-start" />
        <Button className="mt-2 w-full" asChild>
          <a href="#download" onClick={() => setOpen(false)}>
            Download
          </a>
        </Button>
      </div>
    </header>
  );
}
