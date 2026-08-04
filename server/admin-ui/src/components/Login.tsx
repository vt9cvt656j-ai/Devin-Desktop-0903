import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { login } from "@/lib/api";

/**
 * The old login used a verbatim Google "G" logo (admin.html:302) on a product that is not Google,
 * and pasted the same SVG again in the sidebar. Replaced with the wordmark.
 */
export function Login({ onDone }: { onDone: () => void }) {
  const [account, setAccount] = useState("");
  const [password, setPassword] = useState("");
  const [err, setErr] = useState("");
  const [busy, setBusy] = useState(false);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setErr("");
    setBusy(true);
    try {
      await login(account.trim(), password);
      onDone();
    } catch (e) {
      setErr(e instanceof Error ? e.message : "登录失败");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="flex min-h-screen items-center justify-center bg-background px-6">
      <form onSubmit={submit} className="w-full max-w-sm">
        <div className="mb-8 text-center">
          <div className="font-display text-2xl font-bold tracking-tight text-foreground">
            Michael
          </div>
          <p className="type-eyebrow mt-2">管理后台</p>
        </div>
        <div className="rounded-xl border border-border bg-card p-6 shadow-sm">
          <div className="mb-4">
            <Label htmlFor="account">账号</Label>
            <Input
              id="account" value={account} autoFocus autoComplete="username"
              onChange={(e) => setAccount(e.target.value)} placeholder="邮箱或用户名"
            />
          </div>
          <div className="mb-5">
            <Label htmlFor="password">密码</Label>
            <Input
              id="password" type="password" value={password} autoComplete="current-password"
              onChange={(e) => setPassword(e.target.value)} placeholder="••••••••"
            />
          </div>
          {err && (
            <p role="alert" className="mb-4 text-sm text-destructive">{err}</p>
          )}
          <Button type="submit" className="w-full" disabled={busy || !account || !password}>
            {busy ? "登录中…" : "登录"}
          </Button>
        </div>
      </form>
    </div>
  );
}
