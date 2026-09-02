import { useEffect, useState } from "react";
import { Login } from "@/components/Login";
import { Shell, navFor, type NavKey } from "@/components/Shell";
import { Overview } from "@/pages/Overview";
import { Customers } from "@/pages/Customers";
import { Billing } from "@/pages/Billing";
import { Settings } from "@/pages/Settings";
import { RouteEndpoints } from "@/pages/RouteEndpoints";
import { Employees } from "@/pages/Employees";
import { RouteHealth } from "@/pages/RouteHealth";
import { VendorIcons } from "@/pages/VendorIcons";
import { Routing, type RoutingView } from "@/pages/Routing";
import { Pricing } from "@/pages/Pricing";
import { Releases } from "@/pages/Releases";
import { Changelog, type ChangelogView } from "@/pages/Changelog";
import { Docs } from "@/pages/Docs";
import { Commission, type CommissionView } from "@/pages/Commission";
import { Reconcile } from "@/pages/Reconcile";
import { Adapters } from "@/pages/Adapters";
import { RelayRates } from "@/pages/RelayRates";
import { RouteOrder } from "@/pages/RouteOrder";
import { Mail, type MailView } from "@/pages/Mail";
import { api, auth, endConsoleSession } from "@/lib/api";
import { loadSettings } from "@/lib/settings";

/**
 * 「模型线路」下自带接口的那几屏。键就是 NavKey，值是渲染它的那一下。
 *
 * 一张表同时决定两件事：**渲染谁**，以及 Routing 要**排掉谁**。分成两处写过一次，
 * 结果就是加了新屏但忘了排除，两屏叠着渲染（见下面 return 里的注释）。
 */
const ROUTING_OWN_SCREENS = {
  "routing-health": () => <RouteHealth />,
  "routing-endpoints": () => <RouteEndpoints />,
  "routing-icons": () => <VendorIcons />,
  "routing-reconcile": () => <Reconcile view="routing-reconcile" />,
  "routing-reconcile-accounts": () => <Reconcile view="routing-reconcile-accounts" />,
  "routing-sort": () => <RouteOrder />,
  "routing-adapters": () => <Adapters view="routing-adapters" />,
  "routing-adapters-changes": () => <Adapters view="routing-adapters-changes" />,
} as const;

export default function App() {
  const [ready, setReady] = useState(false);
  const [authed, setAuthed] = useState(false);
  const [email, setEmail] = useState("");
  const [page, setPage] = useState<NavKey>("overview");
  /*
   * 结算方式决定侧栏里有没有「提现申请」。在这里取一次，而不是让 Shell 自己去拉 ——
   * 侧栏每一页都在，放在里面就是每次切页都请求一遍同一个设置。
   */
  const [payoutNav, setPayoutNav] = useState({ auto: false, pending: 0 });

  const check = async () => {
    // 没有本地令牌就直接回登录页 —— 不再退回内嵌的登录框。能加载到这段代码说明门禁
    // cookie 还在，而 cookie 在、令牌不在，恰恰是"上一个人退了一半"的状态，必须清干净。
    if (!auth.get()) { void endConsoleSession(); return; }
    try {
      const me = await api.get<{ role?: string; email?: string }>("/api/me");
      if (me?.role !== "admin") {
        // 令牌有效，但这个人不是管理员。这种情况只可能来自"上一段管理员会话还没退，
        // 换了个账号"。不要停在这一屏，把门禁 cookie 和本地令牌一起清掉再回登录页。
        void endConsoleSession();
        return;
      }
      setAuthed(true);
      // 拿不到就按「显示」处理：少显示一个入口比多显示一个更难发现。
      void api
        .get<{ auto_settle: boolean; pending_withdrawals: number }>(
          "/api/admin/referral/settings",
        )
        .then((r) => setPayoutNav({ auto: r.auto_settle, pending: r.pending_withdrawals }))
        .catch(() => undefined);
      // 面值分母等运营参数一次性拉进内存，金额显示才不会停在兜底值上。
      loadSettings(true);
      setEmail(me?.email || "");
    } catch { setAuthed(false); }
    setReady(true);
  };

  useEffect(() => {
    check();
    // One place handles session expiry, rather than every screen inventing its own redirect.
    // 会话过期就回登录页。SPA 内嵌的登录框已经到不了了 —— 没有门禁 cookie 就拿不到
    // 这个 bundle，能看到这段代码本身就说明 cookie 还在。真正过期时要重新签一张。
    const onExpired = () => window.location.replace("/console/login");
    window.addEventListener("admin:unauthorized", onExpired);
    return () => window.removeEventListener("admin:unauthorized", onExpired);
  }, []);

  if (!ready) return <div className="min-h-screen bg-background" />;
  if (!authed) return <Login onDone={check} />;

  return (
    <Shell
      active={page}
      onNavigate={setPage}
      email={email}
      onLogout={() => setAuthed(false)}
      nav={navFor(payoutNav.auto, payoutNav.pending)}
    >
      {page === "overview" && <Overview />}
      {page === "customers" && <Customers />}
      {page === "billing" && <Billing />}
      {page === "settings" && <Settings />}
      {page.startsWith("mail") && <Mail view={page as MailView} />}
      {/*
        「模型线路」下面有几屏是**自成一屏**的：它们自己有接口，不依赖 Routing 那份
        连接数据。这些必须从 Routing 的 `startsWith("routing")` 里排掉，否则两个会
        一起渲染 —— 页面上就是一屏下面又接了一整屏。

        名单**只写一份**。原来是「一行一个 page === 渲染」加「Routing 那行一个手写
        的排除数组」，两处必须同时改；2026-08-25 加「对账」时只改了前一处，线上表现
        是对账页下面又挂了整个线路页。原注释还说「排在前面判掉就行」—— 在同一个 JSX
        片段里那是假的，这些不是 if/else，排除数组才是唯一的闸。
      */}
      {ROUTING_OWN_SCREENS[page as keyof typeof ROUTING_OWN_SCREENS]?.() ??
        (page.startsWith("routing") ? <Routing view={page as RoutingView} /> : null)}
      {page === "employees" && <Employees />}
      {page === "relay-rates" && <RelayRates />}
      {page === "pricing" && <Pricing />}
      {page.startsWith("commission") && <Commission view={page as CommissionView} />}
      {page === "releases" && <Releases />}
      {page === "docs" && <Docs />}
      {page.startsWith("changelog") && <Changelog view={page as ChangelogView} />}
    </Shell>
  );
}
