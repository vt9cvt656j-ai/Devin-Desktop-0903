import { useEffect, useState } from "react";
import { Login } from "@/components/Login";
import { Shell, navFor, type NavKey } from "@/components/Shell";
import { Overview } from "@/pages/Overview";
import { Customers } from "@/pages/Customers";
import { Billing } from "@/pages/Billing";
import { Settings } from "@/pages/Settings";
import { Routing, type RoutingView } from "@/pages/Routing";
import { Pricing } from "@/pages/Pricing";
import { Releases } from "@/pages/Releases";
import { Changelog, type ChangelogView } from "@/pages/Changelog";
import { Docs } from "@/pages/Docs";
import { Commission, type CommissionView } from "@/pages/Commission";
import { Mail, type MailView } from "@/pages/Mail";
import { api, auth, endConsoleSession } from "@/lib/api";
import { loadSettings } from "@/lib/settings";

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
      {page.startsWith("routing") && <Routing view={page as RoutingView} />}
      {page === "pricing" && <Pricing />}
      {page.startsWith("commission") && <Commission view={page as CommissionView} />}
      {page === "releases" && <Releases />}
      {page === "docs" && <Docs />}
      {page.startsWith("changelog") && <Changelog view={page as ChangelogView} />}
    </Shell>
  );
}
