import type { ReactNode } from "react";
import {
  ChevronLeft,
  CreditCard,
  Download,
  Globe,
  Home,
  LineChart,
  LogOut,
  MonitorSmartphone,
  MoreHorizontal,
  Settings,
  Workflow,
} from "lucide-react";

import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuSeparator,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { signOut } from "@/lib/api";
import { DICTS, LANGS, type Lang } from "@/lib/i18n";
import { cn } from "@/lib/utils";

/**
 * One definition of the sidebar, imported by every page.
 *
 * This used to be hand-written markup duplicated per page, and the second copy silently
 * lost two entries — clicking "Plans & billing" made Settings and Integrations vanish.
 * A module cannot drift from itself.
 */
/** The marketing site's download section, which reports what has actually shipped. */
const RELEASES = "https://www.michaelide.xyz/#download";

export const NAV = [
  { href: "/dashboard#overview", key: "navOverview", icon: Home },
  { href: "/dashboard#usage", key: "navUsage", icon: LineChart },
  { href: "/dashboard#settings", key: "navSettings", icon: Settings },
  { href: "/billing", key: "navBilling", icon: CreditCard },
  { separator: true },
  { href: "/dashboard#integrations", key: "navIntegrations", icon: Workflow },
  { href: "/dashboard#devices", key: "navDevices", icon: MonitorSmartphone },
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
  /** Switches the console; the choice is persisted by the caller. */
  onLangChange: (lang: Lang) => void;
  children: ReactNode;
};

export function Shell({
  lang,
  active,
  email,
  name,
  avatar,
  planLabel,
  onLangChange,
  children,
}: ShellProps) {
  const t = DICTS[lang];

  return (
    <div className="flex min-h-screen flex-col md:flex-row">
      {/*
       * Pinned on desktop: `sticky top-0` + `h-screen` makes the column exactly one
       * viewport tall and holds it there while the page scrolls.
       *
       * Without those two the aside is simply the left half of a flex row, so it grows
       * to the height of whatever is beside it. The account block then sits at the
       * bottom of a 2000px *page* rather than the bottom of the *screen* — invisible
       * until you scroll, and sliding away as you do. `mt-auto` was already asking for
       * the bottom; it just had no fixed bottom to be pushed to.
       *
       * Stacked above the content on mobile, where a pinned column would eat the screen.
       */}
      <aside className="flex w-full flex-none flex-col gap-1.5 border-b border-border p-3.5 md:sticky md:top-0 md:h-screen md:w-[264px] md:border-b-0 md:border-r">
        <a
          href="/app/"
          className="mb-3 flex items-center gap-2.5 rounded-lg px-2.5 py-2 text-[13.5px] text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground"
        >
          <ChevronLeft className="size-3.5" />
          {t.openEditor}
        </a>

        {/* Takes the slack so the account block below is pushed to the bottom edge, and
            scrolls on its own if the entries ever outgrow a short window. */}
        <nav className="flex flex-col gap-0.5 md:min-h-0 md:flex-1 md:overflow-y-auto">
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

        <div className="mt-auto flex flex-none flex-col gap-2.5 border-t border-border/60 pt-3">
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

            <DropdownMenu>
              <DropdownMenuTrigger
                aria-label={t.accountMenu}
                className="grid size-7 shrink-0 cursor-pointer place-items-center rounded-lg text-muted-foreground outline-none transition-colors hover:text-foreground focus-visible:text-foreground data-[state=open]:text-foreground"
              >
                <MoreHorizontal className="size-4" />
              </DropdownMenuTrigger>
              {/*
                * Upward, and right-aligned to the trigger.
                *
                * Upward because the trigger sits in the bottom-left corner and a menu
                * anchored below it would run off the window. Right-aligned because
                * `align="start"` hung the panel off the trigger's left edge and out over
                * the page content, which read as belonging to neither the sidebar nor the
                * page; ending it at the same edge as the dots keeps it inside the column
                * it belongs to.
                */}
              <DropdownMenuContent side="top" align="end" className="w-56">
                <DropdownMenuLabel className="truncate">{email}</DropdownMenuLabel>
                <DropdownMenuSeparator />
                {/*
                  * Only destinations that exist. The reference menu carries a changelog,
                  * a help centre and a language switcher; this product has none of the
                  * first two, and the console is English-only by decision — a menu item
                  * that leads nowhere is worse than a shorter menu.
                  */}
                <DropdownMenuItem asChild>
                  <a href="/dashboard#settings">
                    <Settings />
                    {t.navSettings}
                  </a>
                </DropdownMenuItem>
                <DropdownMenuItem asChild>
                  <a href="/billing">
                    <CreditCard />
                    {t.navBilling}
                  </a>
                </DropdownMenuItem>
                <DropdownMenuItem asChild>
                  <a href="/dashboard#devices">
                    <MonitorSmartphone />
                    {t.navDevices}
                  </a>
                </DropdownMenuItem>
                {/*
                  * A submenu rather than a jump to Settings: changing language is a
                  * two-second decision, and sending someone to a page to make it — then
                  * leaving them there — is a worse trade than one extra hover.
                  */}
                <DropdownMenuSub>
                  <DropdownMenuSubTrigger>
                    <Globe />
                    {t.language}
                  </DropdownMenuSubTrigger>
                  {/*
                    * `sideOffset` puts a gap between the two panels — flush against each
                    * other they read as one torn sheet rather than two surfaces.
                    *
                    * `alignOffset` centres the submenu against the parent instead of
                    * hanging it from the row that opened it, which is where Radix puts it
                    * by default and which left it sitting low and lopsided. Measured, not
                    * guessed: the parent is 297px tall, the submenu 266, and the Language
                    * row starts 156px down — so centring means lifting it 156 and pushing
                    * it back down (297-266)/2 = 16, which is -140.
                    *
                    * Both numbers depend on the rows in these two menus. Add a language or
                    * a menu item and it needs re-measuring; the harness that produced these
                    * is a page of the same markup with the built stylesheet.
                    */}
                  <DropdownMenuSubContent sideOffset={8} alignOffset={-140}>
                    <DropdownMenuRadioGroup
                      value={lang}
                      onValueChange={(v) => onLangChange(v as Lang)}
                    >
                      {LANGS.map((l) => (
                        <DropdownMenuRadioItem key={l.value} value={l.value}>
                          {l.label}
                        </DropdownMenuRadioItem>
                      ))}
                    </DropdownMenuRadioGroup>
                  </DropdownMenuSubContent>
                </DropdownMenuSub>
                <DropdownMenuSeparator />
                <DropdownMenuItem asChild>
                  <a href={RELEASES} target="_blank" rel="noreferrer">
                    <Download />
                    {t.getTheApp}
                  </a>
                </DropdownMenuItem>
                <DropdownMenuSeparator />
                <DropdownMenuItem
                  onSelect={() => signOut()}
                  className="text-destructive focus:bg-destructive/10 focus:text-destructive [&_svg]:text-destructive"
                >
                  <LogOut />
                  {t.signOut}
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        </div>
      </aside>

      <main className="min-w-0 flex-1 px-5 pb-16 pt-7 md:px-10">{children}</main>
    </div>
  );
}
