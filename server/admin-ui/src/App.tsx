import { useEffect, useState } from "react";
import { Login } from "@/components/Login";
import { Shell, type NavKey } from "@/components/Shell";
import { Overview } from "@/pages/Overview";
import { Customers } from "@/pages/Customers";
import { Billing } from "@/pages/Billing";
import { Routing } from "@/pages/Routing";
import { Pricing } from "@/pages/Pricing";
import { Releases } from "@/pages/Releases";
import { api, auth } from "@/lib/api";

export default function App() {
  const [ready, setReady] = useState(false);
  const [authed, setAuthed] = useState(false);
  const [email, setEmail] = useState("");
  const [page, setPage] = useState<NavKey>("overview");

  const check = async () => {
    if (!auth.get()) { setAuthed(false); setReady(true); return; }
    try {
      const me = await api.get<{ role?: string; email?: string }>("/api/me");
      setAuthed(me?.role === "admin");
      setEmail(me?.email || "");
    } catch { setAuthed(false); }
    setReady(true);
  };

  useEffect(() => {
    check();
    // One place handles session expiry, rather than every screen inventing its own redirect.
    const onExpired = () => setAuthed(false);
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
      {page === "routing" && <Routing />}
      {page === "pricing" && <Pricing />}
      {page === "releases" && <Releases />}
    </Shell>
  );
}
