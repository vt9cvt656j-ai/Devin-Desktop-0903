import type { ReactNode } from "react";
import { ChevronLeft, CreditCard, Home, LineChart, Settings, Workflow } from "lucide-react";

import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { DICTS, type Lang } from "@/lib/i18n";
import { cn } from "@/lib/utils";

/**
 * One definition of the sidebar, imported by every page.
 *
 * This used to be hand-written markup duplicated per page, and the second copy silently
 * lost two entries — clicking "Plans & billing" made Settings and Integrations vanish.
 * A module cannot drift from itself.
 */
export const NAV = [
  { href: "/dashboard#overview", key: "navOverview", icon: Home },
  { href: "/dashboard#usage", key: "navUsage", icon: LineChart },
  { href: "/dashboard#settings", key: "navSettings", icon: Settings },
  { href: "/billing", key: "navBilling", icon: CreditCard },
  { separator: true },
  { href: "/dashboard#integrations", key: "navIntegrations", icon: Workflow },
] as const;

type ShellProps = {
  lang: Lang;
  /** Path + hash of the active entry, e.g. "/dashboard#usage" or "/billing". */
  active: string;
  email?: string;
  /** Already joined for display, US order. Empty when the account has set no name. */
  name?: string;
  /** `data:` URL, or undefined for the lettered fallback. */
  avatar?: string | null;
  planLabel?: string;
  footer?: ReactNode;
  children: ReactNode;
};

export function Shell({
  lang,
  active,
  email,
  name,
  avatar,
  planLabel,
  footer,
  children,
}: ShellProps) {
  const t = DICTS[lang];

  return (
    <div className="flex min-h-screen flex-col md:flex-row">
      <aside className="flex w-full flex-none flex-col gap-1.5 border-b border-border p-3.5 md:w-[264px] md:border-b-0 md:border-r">
        <a
          href="/app/"
          className="mb-3 flex items-center gap-2.5 rounded-lg px-2.5 py-2 text-[13.5px] text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground"
        >
          <ChevronLeft className="size-3.5" />
          {t.openEditor}
        </a>

        <nav className="flex flex-col gap-0.5">
          {NAV.map((item, i) => {
            if ("separator" in item) return <div key={`sep-${i}`} className="h-3.5" />;
            const Icon = item.icon;
            const on = active === item.href;
            return (
              <a
                key={item.href}
                href={item.href}
                aria-current={on ? "page" : undefined}
                className={cn(
                  "flex items-center gap-3 rounded-lg px-2.5 py-2 text-sm transition-colors hover:bg-secondary",
                  on ? "bg-accent font-semibold" : "font-normal",
                )}
              >
                <Icon className="size-4 opacity-75" />
                {t[item.key]}
              </a>
            );
          })}
        </nav>

        <div className="mt-auto flex flex-col gap-2.5 pt-3">
          {footer}
          <div className="flex items-center gap-2.5 px-1 py-1.5">
            <Avatar className="size-8">
              {avatar ? <AvatarImage src={avatar} alt="" /> : null}
              <AvatarFallback className="bg-primary text-xs font-semibold text-primary-foreground">
                {(name || email || "?").charAt(0).toUpperCase()}
              </AvatarFallback>
            </Avatar>
            {/* Name on top once there is one, with the email demoted beneath it — the
                email is still the thing that identifies the account, so it never
                disappears entirely. */}
            <div className="min-w-0 flex-1">
              <div className="truncate text-[13px] font-medium">{name || email || "—"}</div>
              <div className="truncate text-[11.5px] text-muted-foreground">
                {name ? email : planLabel}
              </div>
            </div>
          </div>
        </div>
      </aside>

      <main className="min-w-0 flex-1 px-5 pb-16 pt-7 md:px-10">{children}</main>
    </div>
  );
}
