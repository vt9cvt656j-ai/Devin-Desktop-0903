import type { ReactNode } from "react";
import { BarChart3, Users, Receipt, Route, Calculator, Package, LogOut } from "lucide-react";
import { cn } from "@/lib/utils";
import { auth } from "@/lib/api";

/**
 * Six destinations, organised by what an operator DOES, not by database table. The old console
 * had nine tabs mirroring nine tables, plus a browser-tab-inside-a-browser-tab strip whose only
 * unique capability was closing every tab to reach an empty placeholder.
 */
export const NAV = [
  { key: "overview", label: "总览", icon: BarChart3 },
  { key: "customers", label: "客户", icon: Users },
  { key: "billing", label: "收款", icon: Receipt },
  { key: "routing", label: "模型线路", icon: Route },
  { key: "pricing", label: "定价试算", icon: Calculator },
  { key: "releases", label: "版本发布", icon: Package },
] as const;

export type NavKey = (typeof NAV)[number]["key"];

export function Shell({
  active, onNavigate, email, onLogout, children,
}: {
  active: NavKey; onNavigate: (k: NavKey) => void;
  email: string; onLogout: () => void; children: ReactNode;
}) {
  return (
    <div className="flex min-h-screen bg-background">
      <aside className="flex w-56 shrink-0 flex-col border-r border-border bg-card">
        <div className="flex h-16 items-center px-5 font-display text-lg font-bold tracking-tight">
          Michael
        </div>
        <nav className="flex-1 space-y-0.5 px-3">
          {NAV.map(({ key, label, icon: Icon }) => (
            <button
              key={key}
              onClick={() => onNavigate(key)}
              aria-current={active === key ? "page" : undefined}
              className={cn(
                "flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-sm font-medium transition-colors",
                active === key
                  ? "bg-secondary text-foreground"
                  : "text-muted-foreground hover:bg-secondary/60 hover:text-foreground",
              )}
            >
              <Icon className="size-4 shrink-0" />
              {label}
            </button>
          ))}
        </nav>
        <div className="border-t border-border p-3">
          <div className="truncate px-2 pb-2 text-xs text-muted-foreground">{email || "—"}</div>
          <button
            onClick={() => { auth.clear(); onLogout(); }}
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
