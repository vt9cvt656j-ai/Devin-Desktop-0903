import type { LucideIcon } from "lucide-react";
import { useState, type ReactNode } from "react";
import {
  BarChart3,
  BookOpen,
  Bot,
  Calculator,
  ChevronRight,
  History,
  LogOut,
  Mail,
  Package,
  Receipt,
  Route,
  Share2,
  SlidersHorizontal,
  Users,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { endConsoleSession } from "@/lib/api";

/**
 * Destinations, organised by what an operator DOES, not by database table. The old console
 * had nine tabs mirroring nine tables, plus a browser-tab-inside-a-browser-tab strip whose only
 * unique capability was closing every tab to reach an empty placeholder.
 */

/**
 * Written out rather than derived from NAV.
 *
 * It used to be `(typeof NAV)[number]["key"]`, which was neat while every entry was a leaf.
 * Once one entry became a group whose own key is not a destination, that derivation would
 * have produced a union containing "commission" — a value nothing renders — while missing
 * the four keys that do. An explicit union is longer and cannot drift into being wrong.
 */
export type NavKey =
  | "overview"
  | "customers"
  | "billing"
  | "settings"
  | "mail"
  | "mail-log"
  | "routing"
  | "routing-health"
  | "routing-groups"
  | "routing-endpoints"
  | "routing-icons"
  | "employees"
  | "pricing"
  | "commission"
  | "commission-pending"
  | "commission-referrers"
  | "commission-referred"
  | "commission-settlements"
  | "commission-withdrawals"
  | "releases"
  | "changelog"
  | "changelog-list"
  | "docs";

type Leaf = { key: NavKey; label: string; icon: LucideIcon };
type Group = {
  /** Not a destination — clicking it opens the group and lands on the first child. */
  group: string;
  label: string;
  icon: LucideIcon;
  children: { key: NavKey; label: string }[];
};

/**
 * 自动结算时「提现申请」不该出现 —— 佣金直接进余额，没有可提的东西。
 *
 * 但只要队列里还有没处理完的申请，就照旧显示：那是切换之前有人提的，钱还等着发。
 * 藏掉它等于把这几个人永远晾在那儿。
 */
export function navFor(autoSettle: boolean, pendingWithdrawals: number): (Leaf | Group)[] {
  const hidePayouts = autoSettle && pendingWithdrawals === 0;
  if (!hidePayouts) return NAV;
  return NAV.map((item) =>
    isGroup(item)
      ? { ...item, children: item.children.filter((c) => c.key !== "commission-withdrawals") }
      : item,
  );
}

export const NAV: (Leaf | Group)[] = [
  { key: "overview", label: "总览", icon: BarChart3 },
  { key: "customers", label: "客户", icon: Users },
  { key: "billing", label: "收款", icon: Receipt },
  { key: "settings", label: "设置", icon: SlidersHorizontal },
  // 紧挨着「客户」和「设置」：群发邮件是对着客户名单做的事，不是一个开关。
  // 写和查是两件事：发一封要专心填内容，翻发送记录是在查某一次发得怎么样。
  {
    group: "mail",
    label: "邮件",
    icon: Mail,
    children: [
      { key: "mail", label: "写一封" },
      { key: "mail-log", label: "发送记录" },
    ],
  },
  // 线路本身和「这些线路在 IDE 里怎么归堆」是两件事：前者是改一条连接的密钥、开放模型和价格，
  // 后者只动模型选择器上的那个标题。混在一张表里，一个纯展示的下拉框看起来就和「停用」一样危险。
  {
    group: "routing",
    label: "模型线路",
    icon: Route,
    children: [
      // 健康放最上面：出事时第一眼看的是「现在能不能用」，不是「怎么配」。
      { key: "routing-health", label: "健康" },
      { key: "routing", label: "线路" },
      { key: "routing-groups", label: "分组" },
      // 一条线路挂多个上游出口。和「线路」是两件事：那里改的是这条线路是什么、卖多少钱，
      // 这里改的只是这一次请求从哪个门发出去 —— 换门换不动账单。
      { key: "routing-endpoints", label: "多路由" },
      // 后台认得出哪些厂商的一张对照表。放在这里是因为它回答的是线路那两屏上的问题：
      // 某条线路的图标是灰的，到底是没这家的图，还是判定没认出来。
      { key: "routing-icons", label: "模型图标" },
    ],
  },
  // 智能员工紧跟在模型线路后面：它干的多数事情就是盯着线路，
  // 而且它用的模型也是上面配的那些线路。
  { key: "employees", label: "智能员工", icon: Bot },
  { key: "pricing", label: "定价试算", icon: Calculator },
  // 分销是四屏，不是一屏：规则、待结算的钱、谁在推荐、被谁推荐来的。挤在一页里
  // 要滚过两块无关内容才能结算一笔佣金。
  {
    group: "commission",
    label: "分销",
    icon: Share2,
    children: [
      { key: "commission", label: "设置" },
      { key: "commission-pending", label: "待结算佣金" },
      { key: "commission-referrers", label: "推荐用户" },
      { key: "commission-referred", label: "被推荐用户" },
      { key: "commission-settlements", label: "结算记录" },
      { key: "commission-withdrawals", label: "提现申请" },
    ],
  },
  { key: "releases", label: "版本发布", icon: Package },
  // 写和看是两件事：发布一条要专心填表，翻已发布的是在找某一条。挤在一页里，
  // 想改上个月那条得先滚过一整张空表单。
  // 文档：一个页面里既写又管（左边编辑、右边列表），不像更新日志那样拆两屏 ——
  // 写文档时经常要照着已有的几页调次序和分组，两边同屏才顺手。
  { key: "docs", label: "用户文档", icon: BookOpen },
  {
    group: "changelog",
    label: "更新日志",
    icon: History,
    children: [
      { key: "changelog", label: "写一条" },
      { key: "changelog-list", label: "已发布" },
    ],
  },
];

const isGroup = (item: Leaf | Group): item is Group => "group" in item;

const rowClass =
  "flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-sm font-medium transition-colors";
const activeClass = "bg-secondary text-foreground";
const idleClass = "text-muted-foreground hover:bg-secondary/60 hover:text-foreground";

function NavGroup({
  item,
  active,
  onNavigate,
}: {
  item: Group;
  active: NavKey;
  onNavigate: (k: NavKey) => void;
}) {
  const holdsActive = item.children.some((c) => c.key === active);
  /*
   * 初始展开与否只看「当前是不是在这一组里」，之后由用户自己控制。
   *
   * 用 useState 的初始值而不是每次渲染都跟着 holdsActive 走：后者等于用户永远关不掉它 ——
   * 点了折叠，下一次渲染又被展开回去。
   */
  const [open, setOpen] = useState(holdsActive);

  return (
    <div>
      <button
        onClick={() => {
          // 从折叠状态点开时顺手落到第一项。展开一个菜单却什么都没发生，
          // 等于要点两次才能到地方。
          if (!open && !holdsActive) onNavigate(item.children[0].key);
          setOpen((v) => !v);
        }}
        aria-expanded={open}
        className={cn(
          rowClass,
          // 组本身不是目的地，所以它只在收起、而组里某一项是当前页时才高亮 ——
          // 否则展开时父项和子项会同时是深色，看不出到底在哪一屏。
          !open && holdsActive ? activeClass : idleClass,
        )}
      >
        <item.icon className="size-4 shrink-0" />
        {item.label}
        <ChevronRight
          aria-hidden
          className={cn(
            "ml-auto size-3.5 transition-transform duration-200",
            open && "rotate-90",
          )}
        />
      </button>

      {open && (
        // 左边那条竖线是缩进的替代品：只靠 padding，四个子项看起来像四个顶级项。
        <div className="ml-[1.4rem] mt-0.5 space-y-0.5 border-l border-border pl-2">
          {item.children.map((c) => (
            <button
              key={c.key}
              onClick={() => onNavigate(c.key)}
              aria-current={active === c.key ? "page" : undefined}
              className={cn(
                "flex w-full items-center rounded-lg px-3 py-2 text-[13px] font-medium transition-colors",
                active === c.key ? activeClass : idleClass,
              )}
            >
              {c.label}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

export function Shell({
  active, onNavigate, email, onLogout, nav = NAV, children,
}: {
  active: NavKey; onNavigate: (k: NavKey) => void;
  email: string; onLogout: () => void;
  /** 默认整份；调用方按结算方式过滤（见 navFor）。 */
  nav?: (Leaf | Group)[];
  children: ReactNode;
}) {
  return (
    <div className="flex min-h-screen bg-background">
      <aside className="sticky top-0 flex h-screen w-56 shrink-0 flex-col border-r border-border bg-card">
        <div className="flex h-16 items-center gap-2.5 px-5">
          <img src="/console/logo.png" alt="" width={24} height={24} className="size-6 rounded-md" />
          <span className="font-display text-base font-bold tracking-tight">Mr. Day One</span>
        </div>
        <nav className="min-h-0 flex-1 space-y-0.5 overflow-y-auto px-3 pb-2">
          {nav.map((item) =>
            isGroup(item) ? (
              <NavGroup key={item.group} item={item} active={active} onNavigate={onNavigate} />
            ) : (
              <button
                key={item.key}
                onClick={() => onNavigate(item.key)}
                aria-current={active === item.key ? "page" : undefined}
                className={cn(rowClass, active === item.key ? activeClass : idleClass)}
              >
                <item.icon className="size-4 shrink-0" />
                {item.label}
              </button>
            ),
          )}
        </nav>
        <div className="mt-auto shrink-0 border-t border-border p-3">
          <div className="truncate px-2 pb-2 text-xs text-muted-foreground">{email || "—"}</div>
          <button
            onClick={() => { onLogout(); void endConsoleSession(); }}
            className="flex w-full items-center gap-2 rounded-lg px-2 py-2 text-sm text-muted-foreground transition-colors hover:bg-secondary/60 hover:text-foreground"
          >
            <LogOut className="size-4" /> 退出登录
          </button>
        </div>
      </aside>
      <main className="min-w-0 flex-1">
        <div className="mx-auto w-full max-w-7xl px-8 py-8">{children}</div>
      </main>
    </div>
  );
}
