import { useCallback, useEffect, useState } from "react";
import { Loader2, Zap } from "lucide-react";

import { Card, CardContent } from "@/components/ui/card";
import { api, type ConnectState } from "@/lib/api";
import { DICTS, type Lang } from "@/lib/i18n";

/*
 * 收款账户。
 *
 * 从「提现」页搬出来的 —— 那一页在开了自动打款之后是藏起来的，而这张卡片恰恰是自动打款
 * 唯一的前提：Stripe Connect 只付给已完成开户的人，账户没绑，每一轮批量打款都会以
 * 「未连接收款账户」跳过，一分钱发不出去。入口不能长在一个会消失的页面上。
 *
 * 放在「邀请」页：那一页在两种打款模式下都在，而且「怎么拿到钱」本来就属于分销的开场说明。
 *
 * 已经绑好并就绪时只留一行状态，不再占一整块 —— 那时候它已经不需要用户做任何事了。
 */
export function ConnectCard({ lang }: { lang: Lang }) {
  const t = DICTS[lang];
  const [state, setState] = useState<ConnectState | null>(null);
  const [linking, setLinking] = useState(false);
  const [err, setErr] = useState("");

  const load = useCallback(async () => {
    // 拿不到就当作没有这回事：开户卡片显示不出来，不该把整页拖垮。
    setState(await api.connect().catch(() => null));
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  // 平台没配 Stripe 时什么都不显示 —— 给一个必然失败的按钮比不给更糟。
  if (!state?.configured) return null;

  if (state.ready) {
    return (
      <Card>
        <CardContent className="flex items-center gap-2 py-4 text-sm text-muted-foreground">
          <Zap className="size-4 shrink-0" />
          {t.connectReady}
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardContent className="flex flex-wrap items-center justify-between gap-4 py-5">
        <div className="min-w-0">
          <div className="flex items-center gap-2 text-sm font-medium">
            <Zap className="size-4" />
            {t.connectTitle}
          </div>
          <p className="mt-1 text-sm text-muted-foreground">
            {state.connected ? t.connectResume : t.connectLede}
          </p>
          {err && <p className="mt-1 text-sm text-destructive">{err}</p>}
        </div>
        <button
          type="button"
          disabled={linking}
          onClick={async () => {
            setLinking(true);
            setErr("");
            try {
              const { url } = await api.connectStart();
              // Stripe 的开户链接是一次性的，直接跳过去；回来时带着 ?connect=done。
              window.location.href = url;
            } catch (e) {
              setErr(e instanceof Error ? e.message : t.withdrawFailed);
              setLinking(false);
            }
          }}
          className="inline-flex shrink-0 items-center gap-2 rounded-lg bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-opacity hover:opacity-90 disabled:opacity-50"
        >
          {linking && <Loader2 className="size-4 animate-spin" />}
          {state.connected ? t.connectResumeCta : t.connectCta}
        </button>
      </CardContent>
    </Card>
  );
}
