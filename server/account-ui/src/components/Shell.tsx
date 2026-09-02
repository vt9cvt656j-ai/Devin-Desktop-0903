import { useState, type ReactNode } from "react";
import {
  ChevronLeft,
  ChevronRight,
  CreditCard,
  Download,
  Globe,
  Home,
  LineChart,
  LogOut,
  Boxes,
  Share2,
  MonitorSmartphone,
  MoreHorizontal,
  Settings,
  Workflow,
} from "lucide-react";

import type { LucideIcon } from "lucide-react";
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
import { DICTS, LANGS, type Dict, type Lang } from "@/lib/i18n";
import { cn } from "@/lib/utils";

/**
 * One definition of the sidebar, imported by every page.
 *
 * This used to be hand-written markup duplicated per page, and the second copy silently
 * lost two entries — clicking "Plans & billing" made Settings and Integrations vanish.
 * A module cannot drift from itself.
 */
/**
 * The marketing site's download section, which reports what has actually shipped.
 *
 * The site moved from www.michaelide.xyz to the apex in August 2026. The old address
 * still 301s here, so an out-of-date copy of this console keeps working — but pointing at
 * the redirect costs every visitor an extra round trip, and michaelide.xyz now means the
 * user-published sites rather than this product.
 */
const RELEASES = "https://mrday.one/#download";

export const NAV = [
  { href: "/dashboard#overview", key: "navOverview", icon: Home },
  { href: "/dashboard#usage", key: "navUsage", icon: LineChart },
  { href: "/dashboard#settings", key: "navSettings", icon: Settings },
  { href: "/billing", key: "navBilling", icon: CreditCard },
  { separator: true },
  { href: "/dashboard#integrations", key: "navIntegrations", icon: Workflow },
  { href: "/dashboard#devices", key: "navDevices", icon: MonitorSmartphone },
  { href: "/dashboard#models", key: "navModels", icon: Boxes },
  /*
   * 佣金是三屏,不是一屏:拿邀请码、看自己带来的人、把钱提出去。挤在一页里,想提现
   * 要先滚过一段和提现无关的东西。
   */
  {
    group: "commission",
    key: "navCommission",
    icon: Share2,
    children: [
      { href: "/dashboard#invite", key: "navInvite" },
      { href: "/dashboard#referrals", key: "navReferrals" },
      { href: "/dashboard#settlements", key: "navSettlements" },
      { href: "/dashboard#withdraw", key: "navWithdraw" },
    ],
  },
] as const;

/**
 * 开了自动打款就藏掉「提现」。
 *
 * 自动打款的意思是：佣金过了冻结期、攒够门槛，系统自己转到对方的 Stripe 账户。这时候
 * 「提现」不是多余，是**没有对应动作** —— 服务端也确实会拒绝手动申请。留着一个点进去
 * 只会看到「已开启自动打款，无需手动申请」的入口，是在浪费用户一次点击。
 *
 * 但有一个前提必须同时成立，否则藏掉它会把整个功能锁死：**「绑定 Stripe」的入口不能只
 * 长在这一页上**。自动打款只付给已完成开户的人，入口一旦不可达，每一轮批量打款都会以
 * 「未连接收款账户」跳过，一分钱发不出去。所以开户卡片已经挪到「邀请」页 —— 那一页在
 * 两种模式下都在。
 *
 * 自己还有没处理完的申请时照旧显示：那笔是切换之前提的，人还在等结果，藏掉等于让他
 * 再也看不到进度。
 */
export function navFor(batchEnabled: boolean, myPendingWithdrawals: number): NavEntry[] {
  const hide = batchEnabled && myPendingWithdrawals === 0;
  if (!hide) return [...NAV];
  return NAV.map((item) =>
    "group" in item
      ? { ...item, children: item.children.filter((c) => c.href !== "/dashboard#withdraw") }
      : item,
  );
}

const navRow =
  "flex items-center gap-3 rounded-lg px-2.5 py-2 text-sm transition-colors hover:bg-secondary";
const navOn = "bg-accent font-semibold";
const navOff = "font-normal";

/*
 * 写出来，而不是从 NAV 用 `as const` 推。
 *
 * `as const` 把 children 推成定长元组，于是「过滤掉提现那一项」得到的数组和它对不上
 * 类型 —— 元组要求正好三项。显式声明既能过滤，也不会因为将来加减一项就崩掉。
 */
type NavKeyOf = keyof Dict;
type NavChild = { href: string; key: NavKeyOf };
type NavGroupItem = {
  group: string;
  key: NavKeyOf;
  icon: LucideIcon;
  children: readonly NavChild[];
};
type NavLeaf = { href: string; key: NavKeyOf; icon: LucideIcon };
type NavEntry = NavLeaf | NavGroupItem | { separator: true };

/**
 * A section that opens to reveal its screens.
 *
 * Opens by itself when you are already inside it, then leaves the state alone — deriving
 * `open` from the active entry on every render would mean a collapse that springs back the
 * moment anything re-renders, which is a control that cannot be used.
 *
 * The header is not a destination: it toggles, and if you open it from cold it also takes
 * you to the first screen. A menu that opens to nothing makes you click twice.
 */
function NavGroup({
  item,
  active,
  t,
}: {
  item: NavGroupItem;
  active: string;
  t: Record<string, string>;
}) {
  const holdsActive = item.children.some((c) => c.href === active);
  const [open, setOpen] = useState(holdsActive);
  const Icon = item.icon;

  return (
    <div>
      <button
        type="button"
        onClick={() => {
          if (!open && !holdsActive) location.hash = item.children[0].href.split("#")[1];
          setOpen((v) => !v);
        }}
        aria-expanded={open}
        className={cn(
          navRow,
          "w-full text-left",
          // Only highlighted while collapsed: with it open, a dark parent and a dark child
          // at once makes it unclear which screen you are actually on.
          !open && holdsActive ? navOn : navOff,
        )}
      >
        <Icon className="size-4 opacity-75" />
        {t[item.key]}
        <ChevronRight
          aria-hidden
          className={cn("ml-auto size-3.5 transition-transform duration-200", open && "rotate-90")}
        />
      </button>

      {open && (
        // The rule down the left is what makes these read as belonging to the section
        // rather than as three more top-level entries.
        <div className="ml-[1.05rem] mt-0.5 flex flex-col gap-0.5 border-l border-border pl-2">
          {item.children.map((c) => (
            <a
              key={c.href}
              href={c.href}
              aria-current={active === c.href ? "page" : undefined}
              className={cn(
                "rounded-lg px-2.5 py-1.5 text-[13px] transition-colors hover:bg-secondary",
                active === c.href ? navOn : navOff,
              )}
            >
              {t[c.key]}
            </a>
          ))}
        </div>
      )}
    </div>
  );
}

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
  /** Defaults to the full set; the caller filters it by settlement mode (see navFor). */
  nav?: readonly NavEntry[];
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
  nav = NAV as readonly NavEntry[],
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
          {nav.map((item, i) => {
            if ("separator" in item) return <div key={`sep-${i}`} className="h-3.5" />;
            if ("group" in item) {
              return <NavGroup key={item.group} item={item} active={active} t={t} />;
            }
            const Icon = item.icon;
            const on = active === item.href;
            return (
              <a
                key={item.href}
                href={item.href}
                aria-current={on ? "page" : undefined}
                className={cn(navRow, on ? navOn : navOff)}
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
                  onSelect={() => void signOut()}
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
