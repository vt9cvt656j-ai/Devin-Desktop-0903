import { useEffect, useState } from "react";
import { Login } from "@/components/Login";
import { Shell, type NavKey } from "@/components/Shell";
import { Overview } from "@/pages/Overview";
import { Customers } from "@/pages/Customers";
import { Billing } from "@/pages/Billing";
import { Settings } from "@/pages/Settings";
import { Routing } from "@/pages/Routing";
import { Pricing } from "@/pages/Pricing";
import { Releases } from "@/pages/Releases";
import { api, auth, endConsoleSession } from "@/lib/api";
import { loadSettings } from "@/lib/settings";

export default function App() {
  const [ready, setReady] = useState(false);
  const [authed, setAuthed] = useState(false);
  const [email, setEmail] = useState("");
  const [page, setPage] = useState<NavKey>("overview");

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
    <Shell active={page} onNavigate={setPage} email={email} onLogout={() => setAuthed(false)}>
      {page === "overview" && <Overview />}
      {page === "customers" && <Customers />}
      {page === "billing" && <Billing />}
      {page === "settings" && <Settings />}
      {page === "routing" && <Routing />}
      {page === "pricing" && <Pricing />}
      {page === "releases" && <Releases />}
    </Shell>
  );
}
