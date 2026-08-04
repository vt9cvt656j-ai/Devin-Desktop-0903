import { useEffect, useMemo, useState } from "react";
import { ErrorState } from "@/components/ErrorState";
import { PageHeader } from "@/components/PageHeader";
import { Panel } from "@/components/Panel";
import { SectionReveal } from "@/components/motion/section-reveal";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { api } from "@/lib/api";
import { cents } from "@/lib/format";
import { applySettings, useSettings, type AdminSettings, type PlanQuota } from "@/lib/settings";

/**
 * 设置 —— 三个原本写死在代码里的运营参数，现在落在 app_settings / plan_quotas 表里。
 *
 * 这一屏之所以来得比想象中晚，是因为面值分母（663）曾经在四个文件里各有一份副本，
 * 其中三份在**写路径**上：运营输入的美元由前端乘 663 变成存库的真实分，服务端不做二次
 * 换算。只把服务端那份改成可配置，"发出去多少额度"会当场和"显示多少"对不上，而且不会
 * 有任何报错。所以先把四份合成一份（服务端下发，见 lib/settings.ts），才有这一屏。
 *
 * 每一项的生效范围都不一样，页面上必须逐条写清楚，因为它们都不可从界面上看出来：
 *  - 面值分母：只改**显示**。已有余额存的是真实分，不会变多也不会变少；改完之后同一笔
 *    余额显示成不同的面值美元。真正受影响的是**之后**发出去的额度。
 *  - 每日赠送：池子按 用户 × 自然日 存，今天已经领过的人要到明天日切才拿到新数。
 *  - 套餐额度：兑换时写进用户表，之后没有任何地方重新推导，所以改了**不会**追改已订阅
 *    的用户。
 */

const MONEY_SAMPLE_CREDIT_USD = 12.34;

/** 面值分母以"分"存储（663），但运营脑子里的单位是"美元"（6.63）。输入框用后者。 */
const toDollars = (rawCents: number) => (rawCents / 100).toFixed(2);
const toRawCents = (dollars: string) => Math.round((Number.parseFloat(dollars) || 0) * 100);

/**
 * 草稿里存的是**美元字符串**，不是分。
 *
 * 之前这三列直接把 5000 / 33000 / 500000 这样的整数分摆进输入框，标题也没有单位。
 * 结果是同一个框可以读成 $5000、$50、或者旁边那列印的 $7.54 —— 看起来像点数刻度，
 * 而这几个数是真金白银。控制台里其他每一个金额输入框（充值、赠送额度）都是美元，
 * 这里也必须是。
 */
type PlanDraft = Record<string, { total: string; window: string; weekly: string; days: string }>;

function planDraftFrom(plans: PlanQuota[]): PlanDraft {
  const d: PlanDraft = {};
  for (const p of plans) {
    d[p.plan] = {
      total: toDollars(p.total_cents),
      window: toDollars(p.window_cents),
      weekly: toDollars(p.weekly_cents),
      days: String(p.days),
    };
  }
  return d;
}

function Hint({ children }: { children: React.ReactNode }) {
  return <p className="mt-2 text-xs leading-relaxed text-muted-foreground">{children}</p>;
}

export function Settings() {
  const live = useSettings();
  const [loading, setLoading] = useState(true);
  const [err, setErr] = useState("");
  const [msg, setMsg] = useState<{ text: string; ok: boolean } | null>(null);
  const [busy, setBusy] = useState(false);

  const [denomInput, setDenomInput] = useState(toDollars(live.raw_cents_per_credit_usd));
  const [freeInput, setFreeInput] = useState(String(live.free_points_daily));
  const [plans, setPlans] = useState<PlanDraft>({});
  const [confirmDenom, setConfirmDenom] = useState(false);
  const [confirmText, setConfirmText] = useState("");

  const refresh = async () => {
    setLoading(true);
    setErr("");
    try {
      const s = await api.get<AdminSettings>("/api/admin/settings");
      applySettings(s);
      setDenomInput(toDollars(s.raw_cents_per_credit_usd));
      setFreeInput(String(s.free_points_daily));
      setPlans(planDraftFrom(s.plans || []));
    } catch (e) {
      setErr(e instanceof Error ? e.message : "读取设置失败");
    }
    setLoading(false);
  };

  useEffect(() => {
    refresh();
  }, []);

  const savedDenom = live.raw_cents_per_credit_usd;
  const draftDenom = toRawCents(denomInput);
  const denomValid =
    draftDenom >= live.limits.raw_cents_per_credit_usd[0] &&
    draftDenom <= live.limits.raw_cents_per_credit_usd[1];
  const denomChanged = denomValid && draftDenom !== savedDenom;

  /**
   * 影响面预览。用一笔固定的真实分（现在正好显示成 $12.34）算出改完之后显示成多少 ——
   * 真实分一分没动，动的只是它显示成几美元。这是这个数唯一会立刻改变的东西。
   */
  const preview = useMemo(() => {
    const rawCents = Math.round(MONEY_SAMPLE_CREDIT_USD * savedDenom);
    const after = draftDenom > 0 ? Math.round((rawCents / draftDenom) * 100) : 0;
    return { rawCents, before: Math.round(MONEY_SAMPLE_CREDIT_USD * 100), after };
  }, [savedDenom, draftDenom]);

  const save = async (body: Record<string, unknown>, okText: string) => {
    setBusy(true);
    setMsg(null);
    try {
      const r = await api.post<AdminSettings>("/api/admin/settings", body);
      applySettings(r);
      setDenomInput(toDollars(r.raw_cents_per_credit_usd));
      setFreeInput(String(r.free_points_daily));
      if (r.plans) setPlans(planDraftFrom(r.plans));
      setMsg({ text: okText, ok: true });
    } catch (e) {
      setMsg({ text: e instanceof Error ? e.message : "保存失败", ok: false });
    }
    setBusy(false);
  };

  const savePlans = () => {
    const list = Object.entries(plans).map(([plan, d]) => ({
      plan,
      total_cents: toRawCents(d.total),
      window_cents: toRawCents(d.window),
      weekly_cents: toRawCents(d.weekly),
      days: Number.parseInt(d.days, 10) || 0,
    }));
    const bad = list.find((p) => p.window_cents <= 0);
    if (bad) {
      setMsg({
        text: `${bad.plan}：时段上限必须大于 0 —— 填 0 不是"不限"，是把这个套餐永久锁死`,
        ok: false,
      });
      return;
    }
    save({ plans: list }, "套餐额度已保存（只影响之后的开通与兑换）");
  };

  if (err) return <ErrorState message={err} onRetry={refresh} />;

  return (
    <div className="space-y-6">
      <PageHeader
        title="设置"
        description="面值、每日赠送、套餐额度。改这里之前先看清每一项的生效范围 —— 三项都不一样。"
        actions={
          <Button variant="outline" onClick={refresh} disabled={loading || busy}>
            {loading ? "读取中…" : "重新读取"}
          </Button>
        }
      />

      {msg && (
        <SectionReveal>
          <div
            className={`rounded-lg border px-4 py-3 text-sm ${
              msg.ok
                ? "border-border bg-secondary/40 text-foreground"
                : "border-border bg-secondary/40 text-foreground"
            }`}
          >
            <Badge variant={msg.ok ? "success" : "outline"}>{msg.ok ? "已保存" : "未保存"}</Badge>
            <span className="ml-2">{msg.text}</span>
          </div>
        </SectionReveal>
      )}

      <SectionReveal delay={70}>
        <Panel title="额度面值（混合汇率）" bodyClassName="p-5">
          <div className="grid gap-6 lg:grid-cols-[minmax(0,20rem)_1fr]">
            <div>
              <Label htmlFor="denom">1 美元面值额度 = 多少美元真实成本</Label>
              <Input
                id="denom"
                inputMode="decimal"
                value={denomInput}
                onChange={(e) => setDenomInput(e.target.value)}
                className="mt-1.5"
              />
              <Hint>
                当前 <span className="tabular-nums">{toDollars(savedDenom)}</span>（
                {savedDenom} 真实计费分 = 客户看到的 $1.00）。这是卖出的 1 美元额度对应的上游
                真实成本，不是人民币汇率 —— 渠道的购买价在「模型线路」里按渠道单独填。
              </Hint>
              <Button
                className="mt-3"
                disabled={!denomChanged || busy}
                onClick={() => {
                  setConfirmText("");
                  setConfirmDenom(true);
                }}
              >
                修改面值…
              </Button>
              {!denomValid && (
                <Hint>
                  需在 {toDollars(live.limits.raw_cents_per_credit_usd[0])} ~{" "}
                  {toDollars(live.limits.raw_cents_per_credit_usd[1])} 之间。
                </Hint>
              )}
            </div>

            <div className="rounded-xl border border-border bg-secondary/40 p-4">
              <h3 className="text-sm font-semibold">改完之后会发生什么</h3>
              <dl className="mt-3 space-y-3 text-sm">
                <div className="flex items-baseline justify-between gap-4">
                  <dt className="text-muted-foreground">一笔现在显示 {cents(preview.before)} 的余额</dt>
                  <dd className="tabular-nums font-medium">
                    {denomChanged ? `${cents(preview.before)} → ${cents(preview.after)}` : cents(preview.before)}
                  </dd>
                </div>
                <div className="flex items-baseline justify-between gap-4">
                  <dt className="text-muted-foreground">它存的真实计费分</dt>
                  <dd className="tabular-nums font-medium">
                    {preview.rawCents} → {preview.rawCents}
                  </dd>
                </div>
              </dl>
              <Hint>
                <strong className="font-medium text-foreground">能花的钱一分没变</strong>
                ：余额、套餐额度都是按真实计费分存的，结算也按真实分扣，改面值不碰它们。变的是
                同一笔真实分显示成多少面值美元 —— 客户会看到余额数字变了，却没有发生任何交易。
              </Hint>
              <Hint>
                真正受影响的是<strong className="font-medium text-foreground">之后</strong>发出去的额度：
                同样标价的商品，改完之后卖出去的那份真实额度不一样。已创建的商品、未支付的订单、
                未兑换的兑换码，真实分在创建时就冻结了，不会被追改。
              </Hint>
            </div>
          </div>
        </Panel>
      </SectionReveal>

      <SectionReveal delay={140}>
        <Panel title="每日赠送" bodyClassName="p-5">
          <div className="grid gap-6 lg:grid-cols-[minmax(0,20rem)_1fr]">
            <div>
              <Label htmlFor="free">每人每天赠送点数</Label>
              <Input
                id="free"
                inputMode="numeric"
                value={freeInput}
                onChange={(e) => setFreeInput(e.target.value)}
                className="mt-1.5"
              />
              <Hint>
                当前 {live.free_points_daily} 点。运营按点定价：¥0.5 = 10 点，所以 1 点 = ¥0.05，
                {live.free_points_daily} 点即每天 ¥{(live.free_points_daily * 0.05).toFixed(2)}。
                填 0 等于关掉免费额度。
              </Hint>
              <Button
                className="mt-3"
                disabled={busy || String(live.free_points_daily) === freeInput.trim()}
                onClick={() =>
                  save(
                    { free_points_daily: Number.parseInt(freeInput, 10) || 0 },
                    "每日赠送已保存（明天日切后对所有人生效）",
                  )
                }
              >
                保存
              </Button>
            </div>
            <div className="rounded-xl border border-border bg-secondary/40 p-4">
              <h3 className="text-sm font-semibold">生效时间</h3>
              <Hint>
                免费池是按 <strong className="font-medium text-foreground">用户 × 自然日</strong> 存下来的：
                只有当用户当天第一次调用时，才会把当天的额度写进去。所以今天已经用过的人，池子里
                还是今天发的那份，改完不会立刻变多或变少 —— 新数字在他们
                <strong className="font-medium text-foreground">下一次日切之后</strong>的第一次调用生效。
              </Hint>
              <Hint>
                这一项不改变任何人被收多少钱：免费点走的是免费池分支，扣不到余额上。它决定的是
                你每天送出去多少真实上游消耗。
              </Hint>
            </div>
          </div>
        </Panel>
      </SectionReveal>

      <SectionReveal delay={210}>
        <Panel
          title="套餐额度"
          aside={
            <Button onClick={savePlans} disabled={busy || loading}>
              保存套餐
            </Button>
          }
        >
          <div className="overflow-x-auto">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="w-32">套餐</TableHead>
                  <TableHead>总额度（真实计费 $）</TableHead>
                  <TableHead>时段上限 5.5h（真实计费 $）</TableHead>
                  <TableHead>周上限（真实计费 $）</TableHead>
                  <TableHead className="w-24">天数</TableHead>
                  <TableHead className="w-52">客户看到（总 / 时段 / 周）</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {(live.plans || []).map((p) => {
                  const d = plans[p.plan] || { total: "", window: "", weekly: "", days: "" };
                  const set = (k: keyof typeof d, v: string) =>
                    setPlans((prev) => ({ ...prev, [p.plan]: { ...prev[p.plan], [k]: v } }));
                  // 客户看到的是面值，三列都要折算 —— 有争议的正是时段/周这两列。
                  const face = (dollars: string) =>
                    cents(Math.round((toRawCents(dollars) / (savedDenom || 663)) * 100));
                  return (
                    <TableRow key={p.plan}>
                      <TableCell className="font-medium">
                        {p.plan}
                        {p.is_default && (
                          <div className="mt-1">
                            <Badge variant="outline">出厂默认</Badge>
                          </div>
                        )}
                      </TableCell>
                      <TableCell>
                        <Input inputMode="decimal" value={d.total} onChange={(e) => set("total", e.target.value)} />
                      </TableCell>
                      <TableCell>
                        <Input inputMode="decimal" value={d.window} onChange={(e) => set("window", e.target.value)} />
                      </TableCell>
                      <TableCell>
                        <Input inputMode="decimal" value={d.weekly} onChange={(e) => set("weekly", e.target.value)} />
                        {toRawCents(d.weekly) === 0 && (
                          <div className="mt-1 text-xs text-muted-foreground">0 = 不限</div>
                        )}
                      </TableCell>
                      <TableCell>
                        <Input inputMode="numeric" value={d.days} onChange={(e) => set("days", e.target.value)} />
                      </TableCell>
                      <TableCell className="tabular-nums text-xs text-muted-foreground">
                        <div>{face(d.total)}</div>
                        <div>{face(d.window)}</div>
                        <div>{toRawCents(d.weekly) === 0 ? "不限" : face(d.weekly)}</div>
                      </TableCell>
                    </TableRow>
                  );
                })}
              </TableBody>
            </Table>
          </div>
          <div className="border-t border-border p-5">
            <Hint>
              输入的是<strong className="font-medium text-foreground">真实计费美元</strong>
              —— 你实际付给上游的钱，和钱包余额同一个单位（结算时同一笔费用拆开，一部分扣套餐额度，
              一部分扣钱包）。右侧「客户看到」是同一笔钱按当前面值折算后、客户端上显示的数字。
              两个都是美元，差 {(savedDenom / 100).toFixed(2)} 倍，别看混。
            </Hint>
            <Hint>
              <strong className="font-medium text-foreground">同样是 0，两列的含义相反。</strong>
              周上限填 0 = <strong className="font-medium text-foreground">不限</strong>；
              时段上限填 0 = 把套餐<strong className="font-medium text-foreground">永久锁死</strong>
              （判定用的是「大于 0」），用户会一直看到「本时段额度已用完，请等待刷新」，而那个刷新
              永远刷不出额度。所以时段上限被数据库拦着，不允许填 0。
            </Hint>
            <Hint>
              改套餐<strong className="font-medium text-foreground">不会</strong>动已订阅的用户：额度在兑换那一刻
              写进用户表，之后没有任何地方重新推导。只有在「客户」页手动改套餐并勾上「重置额度」时才会重写。
            </Hint>
          </div>
        </Panel>
      </SectionReveal>

      <SectionReveal delay={280}>
        <Panel title="不在这里改的" bodyClassName="p-5">
          <dl className="grid gap-4 sm:grid-cols-2">
            <div>
              <dt className="text-sm font-medium">1 点 = {live.raw_cents_per_point} 真实计费分</dt>
              <dd className="mt-1 text-xs leading-relaxed text-muted-foreground">
                编译期常量，由面值和点价手工推导而来。它是每次调用的换算除数，改错会立刻影响所有
                在线用户的扣费，所以刻意留在代码里。
              </dd>
            </div>
            <div>
              <dt className="text-sm font-medium">渠道购买价</dt>
              <dd className="mt-1 text-xs leading-relaxed text-muted-foreground">
                每条中转线路各自的进货价（¥1 买多少美元额度），在「模型线路」里按渠道填 —— 它和这一屏
                的面值是两个数，不要混。
              </dd>
            </div>
          </dl>
        </Panel>
      </SectionReveal>

      <Dialog open={confirmDenom} onOpenChange={setConfirmDenom}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>确认修改额度面值</DialogTitle>
            <DialogDescription>
              这会改变每一个客户看到的余额数字，而他们并没有发生任何交易。
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-3 text-sm">
            <div className="rounded-lg border border-border bg-secondary/40 p-3">
              <div className="flex items-baseline justify-between gap-4">
                <span className="text-muted-foreground">面值</span>
                <span className="tabular-nums font-medium">
                  {toDollars(savedDenom)} → {toDollars(draftDenom)}
                </span>
              </div>
              <div className="mt-2 flex items-baseline justify-between gap-4">
                <span className="text-muted-foreground">一笔 {cents(preview.before)} 的余额会显示成</span>
                <span className="tabular-nums font-medium">{cents(preview.after)}</span>
              </div>
              <div className="mt-2 flex items-baseline justify-between gap-4">
                <span className="text-muted-foreground">它能花的真实金额</span>
                <span className="tabular-nums font-medium">不变</span>
              </div>
            </div>
            <p className="text-xs leading-relaxed text-muted-foreground">
              之后发出去的额度会按新面值折算；已创建的商品、未支付的订单和未兑换的兑换码保持原有的
              真实分不变。
            </p>
            <div>
              <Label htmlFor="confirm-denom">输入「确认」以继续</Label>
              <Input
                id="confirm-denom"
                value={confirmText}
                onChange={(e) => setConfirmText(e.target.value)}
                className="mt-1.5"
              />
            </div>
          </div>
          <div className="flex justify-end gap-2">
            <Button variant="outline" onClick={() => setConfirmDenom(false)}>
              取消
            </Button>
            <Button
              disabled={confirmText.trim() !== "确认" || busy}
              onClick={async () => {
                setConfirmDenom(false);
                await save({ raw_cents_per_credit_usd: draftDenom }, "额度面值已更新");
              }}
            >
              确认修改
            </Button>
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}
