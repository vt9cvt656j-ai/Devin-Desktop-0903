import { useCallback, useEffect, useState, type Dispatch, type SetStateAction } from "react";
import { Check, ChevronLeft, ChevronRight, Link2, Loader2, Share2, UserPlus, Users, Wallet, X } from "lucide-react";

import { EmptyState } from "@/components/EmptyState";
import { ErrorState } from "@/components/ErrorState";
import { PageHeader } from "@/components/PageHeader";
import { Panel } from "@/components/Panel";
import { TableSkeleton } from "@/components/TableSkeleton";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
  Truncate,
} from "@/components/ui/table";
import { api } from "@/lib/api";
import { cn } from "@/lib/utils";

/**
 * 分销 —— 四屏,由侧栏的展开菜单切换。
 *
 * 一屏一件事:定规则、结算钱、看谁在推荐、看被谁推荐来的。
 *
 * 三张列表用的是 ui/table,不是手搓的 flex 行。第一版是手搓的,空数据下看着没问题,
 * 装上真数据立刻现原形:一个 84 字符的邮箱把徽章和金额顶到面板边缘外,金额右对齐了
 * 却因为徽章宽度不同而每行错开一点,四个数字挤在一行里没有表头说明哪个是哪个。
 * 这些问题 ui/table 那一层早就解决过一遍 —— 固定列宽、numeric 右对齐 + 等宽数字、
 * Truncate 把截断和 title 绑在一起、窄于下限就横向滚动而不是把格子压成竖排。
 *
 * 后两屏是同一批数据的两个方向:「推荐用户」按推荐人聚合,「被推荐用户」一行一个被
 * 推荐的人。前者回答"谁真的在带人进来",后者回答"这个客户是从哪来的" —— 一个带来
 * 四十个注册却没有一分钱的推荐人,在后者的列表里看不出来。
 *
 * 有一条规则界面上写死了,因为它就是实际行为:**改比例和期限只影响之后新绑定的推荐
 * 关系**。已经绑定的,比例和到期时间是绑定当时冻结下来的。
 */

export type CommissionView =
  | "commission"
  | "commission-pending"
  | "commission-referrers"
  | "commission-referred"
  | "commission-settlements"
  | "commission-withdrawals";

type Settings = {
  rate_bps: number;
  window_days: number;
  enabled: boolean;
  /** true = 佣金直接进对方余额；false = 记一笔待结算，人工转账。 */
  auto_settle: boolean;
  /** 佣金审核通过后要冻结多少天才允许打款。挡的是退款和拒付。 */
  hold_days: number;
  /** 同一个推荐人攒够多少分才发一笔，免得手续费吃掉小额佣金。 */
  min_payout_cents: number;
  /** 定时批量打款的总开关。开了之后服务器会自动往外转钱。 */
  batch_enabled: boolean;
  /** 还在冻结期里的钱 / 已到期正在等门槛的钱。 */
  holding_cents: number;
  ready_cents: number;
  referrals: number;
  active: number;
  pending_withdrawals: number;
};

type ReferredRow = {
  referrer_email: string;
  referred_email: string;
  code: string;
  source: string;
  rate_bps: number;
  created_at: string;
  expires_at: string;
  active: boolean;
  earned_cents: number;
};

type ReferrerRow = {
  id: string;
  email: string;
  referral_enabled: boolean;
  code: string;
  invited: number;
  active: number;
  pending_cents: number;
  settled_cents: number;
  last_at: string | null;
};

type ReferrerList = {
  rows: ReferrerRow[];
  /** The page actually served — a clamped request comes back as where it really is. */
  page: number;
  pages: number;
  /** Counted over the whole (filtered) list, not the page. */
  total: number;
  granted: number;
  per_page: number;
};

type Commission = {
  id: string;
  referrer_email: string;
  customer_email: string;
  source: string;
  amount_cents: number;
  rate_bps: number;
  commission_cents: number;
  status: string;
  note: string;
  created_at: string;
};

type AdminWithdrawal = {
  id: string;
  email: string;
  name: string;
  amount_cents: number;
  method: string;
  account: string;
  qr: string | null;
  status: string;
  note: string;
  settled_cents: number;
  created_at: string;
  paid_at: string | null;
  /** 谁点的「已支付」，以及这笔转账自己的流水号。付款之前都是空的。 */
  paid_by: string;
  reference: string;
  /** 'manual' 或 'stripe_connect' */
  provider?: string;
  /** Stripe 的 tr_… */
  transfer_id?: string | null;
  /** 自动打款没走成的原因（Stripe 原话）。status=sending 且有这个值 = 结果不明。 */
  failure_reason?: string;
};

type WithdrawList = {
  rows: AdminWithdrawal[];
  /** The page actually served — a clamped request comes back as where it really is. */
  page: number;
  pages: number;
  total: number;
  per_page: number;
  pending_total_cents: number;
};

type Settlement = {
  id: string;
  referrer_email: string;
  customer_email: string;
  amount_cents: number;
  rate_bps: number;
  commission_cents: number;
  /** 'auto' | 操作员邮箱 | ''(这列存在之前结算的老数据) */
  settled_by: string;
  settled_at: string | null;
  created_at: string;
  /** 'settled' | 'reversed'（付款被退了，但钱还没发出去，所以直接撤销） */
  status: string;
  /** 有值就说明这笔订单退款或被拒付了。status 仍是 settled 的那些是钱已经发出去的，要人工处理。 */
  reversed_at: string | null;
  reversal_reason: string;
};

type SettlementList = {
  rows: Settlement[];
  page: number;
  pages: number;
  total: number;
  auto_count: number;
  manual_count: number;
  total_cents: number;
  /** 已结算之后才退款的笔数 —— 钱已经出去了，软件收不回来。 */
  flagged: number;
  per_page: number;
};

type CommissionList = {
  rows: Commission[];
  summary: { pending_cents: number; settled_cents: number; total_cents: number };
};

const usd = (cents: number) =>
  (cents / 100).toLocaleString("en-US", { style: "currency", currency: "USD" });

/** 比例只在需要时带小数:30% 不写成 30.00%,12.5% 也不被截成 13%。 */
const pct = (bps: number) => `${Number((bps / 100).toFixed(2))}%`;

function day(iso: string | null): string {
  if (!iso) return "—";
  const d = new Date(iso);
  // 拿不出日期就还它原样 —— 显示一个"Invalid Date"比显示原始字符串更没用。
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleDateString("zh-CN");
}

const METHOD_LABEL: Record<string, string> = {
  alipay: "支付宝",
  wechat: "微信支付",
  bank: "银行卡",
  paypal: "PayPal",
};

const PAYOUT_STYLE: Record<string, string> = {
  pending: "bg-amber-100 text-amber-700 dark:bg-amber-950 dark:text-amber-400",
  // 转账已经发给 Stripe，结果还没回来。红色不是因为出错，是因为这一行**绝对不能手工再付一次**。
  sending: "bg-red-100 text-red-700 dark:bg-red-950 dark:text-red-400",
  paid: "bg-emerald-100 text-emerald-700 dark:bg-emerald-950 dark:text-emerald-400",
  rejected: "bg-muted text-muted-foreground",
  failed: "bg-amber-100 text-amber-700 dark:bg-amber-950 dark:text-amber-400",
  returned: "bg-amber-100 text-amber-700 dark:bg-amber-950 dark:text-amber-400",
};

const PAYOUT_LABEL: Record<string, string> = {
  pending: "待处理",
  sending: "转账中",
  paid: "已支付",
  rejected: "已驳回",
  failed: "转账失败",
  returned: "已退回",
};

const HEADING: Record<CommissionView, { title: string; description: string }> = {
  commission: {
    title: "分销 · 设置",
    description: "返佣比例和期限。改动只影响之后新绑定的推荐关系,已绑定的不变。",
  },
  "commission-pending": {
    title: "分销 · 待结算佣金",
    description: "被推荐的用户在 Stripe 付款成功时自动记一笔,在这里结算或驳回。",
  },
  "commission-referrers": {
    title: "分销 · 推荐用户",
    description:
      "给账号开通推荐资格。开通后对方就有了自己的邀请码和链接,可以去带人;收回资格只是不再接受新的绑定,已经绑定的继续返佣到期满。",
  },
  "commission-referred": {
    title: "分销 · 被推荐用户",
    description: "每个被推荐进来的账号:谁推荐的、用的哪个码、返佣期到什么时候。",
  },
  "commission-settlements": {
    title: "分销 · 结算记录",
    description:
      "已经结算掉的每一笔佣金 —— 自动结算的直接进了对方余额，人工结算的记着是谁点的。",
  },
  "commission-withdrawals": {
    title: "分销 · 提现申请",
    description:
      // 不说转账方式：它由 app_settings.referral_batch_enabled 决定，线上是**自动**
      // （Stripe Connect 批量转账），而这句写死的话一直在说人工。与其从服务端下发一个
      // 只为这句话服务的字段，不如别做这个断言 —— 这一屏本来就是「谁要提、提多少、转到哪」。
      "谁要把佣金提出去、提多少、转到哪里。",
  },
};

/**
 * 一页装不下才出现。只有一页时它是两个按不动的按钮。
 *
 * `page` 用服务端回的那个,不是本地 state:请求越界时后端会夹到最后一页,照着本地
 * 的数字画会显示一个并不存在的页码。
 */
function Pager({
  page,
  pages,
  total,
  onPage,
}: {
  page: number;
  pages: number;
  total: number;
  /** The setter itself, so clicks compose — see below. */
  onPage: Dispatch<SetStateAction<number>>;
}) {
  if (pages <= 1) return null;
  /*
   * 用函数式更新，而不是 onPage(page + 1)。
   *
   * page 是服务端回的那个,一次请求回来之前它不会变 —— 连点两下「下一页」的两次
   * 计算都读到同一个 1,于是点两下只前进一页。改成基于上一个值累加,连点就真的连翻,
   * 上限用上一次响应里的 pages 夹住。
   */
  return (
    <div className="flex items-center gap-2 border-t border-border px-5 py-3 text-xs text-muted-foreground">
      <Button
        size="sm"
        variant="outline"
        disabled={page <= 1}
        onClick={() => onPage((p) => Math.max(1, p - 1))}
      >
        <ChevronLeft /> 上一页
      </Button>
      <span className="tabular-nums">
        第 {page} / {pages} 页 · 共 {total} 条
      </span>
      <Button
        size="sm"
        variant="outline"
        disabled={page >= pages}
        onClick={() => onPage((p) => Math.min(pages, p + 1))}
      >
        下一页 <ChevronRight />
      </Button>
    </div>
  );
}

export function Commission({ view }: { view: CommissionView }) {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [referred, setReferred] = useState<ReferredRow[] | null>(null);
  const [referrers, setReferrers] = useState<ReferrerList | null>(null);
  const [referrerPage, setReferrerPage] = useState(1);
  const [ledger, setLedger] = useState<CommissionList | null>(null);
  const [payouts, setPayouts] = useState<WithdrawList | null>(null);
  /** 放大看的收款码。列表里的缩略图小到扫不了。 */
  const [zoom, setZoom] = useState<string | null>(null);
  const [payoutPage, setPayoutPage] = useState(1);
  const [settlements, setSettlements] = useState<SettlementList | null>(null);
  const [settlementPage, setSettlementPage] = useState(1);
  const [error, setError] = useState<string | null>(null);

  // 表单单独存:比例用百分比、期限用天,和后端的 bps / days 分开,免得每次输入都换算。
  const [ratePct, setRatePct] = useState("30");
  const [days, setDays] = useState("90");
  const [enabled, setEnabled] = useState(true);
  const [autoSettle, setAutoSettle] = useState(false);
  const [holdDays, setHoldDays] = useState("14");
  const [minPayout, setMinPayout] = useState("50");
  const [batchEnabled, setBatchEnabled] = useState(false);
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState<{ text: string; ok: boolean } | null>(null);
  /** 推荐用户那一屏的搜索。账号列表可能有几百行,翻是翻不动的。 */
  const [query, setQuery] = useState("");

  /*
   * 四屏各取各的。以前一次拉三个接口,不管当前在哪一屏 —— 只想调个比例,却顺带
   * 把两百条推荐关系和整本佣金账拉了下来。
   */
  const load = useCallback(async () => {
    try {
      if (view === "commission") {
        const s = await api.get<Settings>("/api/admin/referral/settings");
        setSettings(s);
        setRatePct(String(s.rate_bps / 100));
        setDays(String(s.window_days));
        setEnabled(s.enabled);
        setAutoSettle(s.auto_settle);
        setHoldDays(String(s.hold_days));
        setMinPayout(String(s.min_payout_cents / 100));
        setBatchEnabled(s.batch_enabled);
      } else if (view === "commission-pending") {
        setLedger(await api.get<CommissionList>("/api/admin/commissions"));
      } else if (view === "commission-referrers") {
        setReferrers(
          await api.get<ReferrerList>(
            `/api/admin/referral/referrers?q=${encodeURIComponent(query.trim())}&page=${referrerPage}`,
          ),
        );
      } else if (view === "commission-settlements") {
        setSettlements(
          await api.get<SettlementList>(`/api/admin/settlements?page=${settlementPage}`),
        );
      } else if (view === "commission-withdrawals") {
        setPayouts(
          await api.get<WithdrawList>(
            `/api/admin/withdrawals?status=all&page=${payoutPage}`,
          ),
        );
      } else {
        setReferred(await api.get<ReferredRow[]>("/api/admin/referral/list"));
      }
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "加载失败");
    }
  }, [view, query, referrerPage, payoutPage, settlementPage]);

  useEffect(() => {
    // 搜索框每敲一个字都发一次请求太吵,停手 250ms 再查。
    const t = setTimeout(() => void load(), query ? 250 : 0);
    return () => clearTimeout(t);
  }, [load, query]);

  // 换一屏就把上一屏的提示收掉,免得一条旧错误跟着人走。
  useEffect(() => {
    setNote(null);
  }, [view]);

  // 搜索换了就回第一页。留在第 4 页去搜一个只有两条结果的关键词,后端会夹到第 1 页,
  // 但本地还记着 4 —— 下一次翻页就从一个不存在的位置开始算。
  useEffect(() => {
    setReferrerPage(1);
  }, [query]);

  async function grant(r: ReferrerRow, enabled: boolean) {
    // 收回资格不影响已经绑定的推荐关系,所以不值得拦一道确认;开通更不用。
    try {
      const res = await api.post<{ enabled: boolean; code: string | null }>(
        `/api/admin/referral/grant/${r.id}`,
        { enabled },
      );
      setNote({
        text: enabled
          ? `已给 ${r.email} 开通推荐资格${res.code ? `，邀请码 ${res.code}` : ""}。`
          : `已收回 ${r.email} 的推荐资格。已经绑定的推荐关系不受影响。`,
        ok: true,
      });
      await load();
    } catch (e) {
      setNote({ text: e instanceof Error ? e.message : "操作失败", ok: false });
    }
  }

  async function decide(w: AdminWithdrawal, status: "paid" | "rejected") {
    const verb = status === "paid" ? "标记为已支付" : "驳回";
    let reference = "";
    if (status === "paid") {
      /*
       * 流水号是这条记录里唯一能和真实转账对上的东西。三个月后有人问「你们到底发了没有」，
       * 一个状态字段回答不了，一个支付宝订单号可以。
       *
       * 用 prompt 而不是再开一个对话框：这一步在确认之后、写库之前，中间插一个表单会让
       * 「我到底点没点确认」变得不清楚。留空也放行 —— 有些转账方式确实没有单号，挡住
       * 只会逼人随便填一个。
       */
      reference =
        prompt(
          `已经把 ${usd(w.amount_cents)} 转给 ${w.email} 了吗？\n\n` +
            `填上转账流水号（银行回单号、支付宝订单号等），以后对账要用。没有就留空。\n` +
            `标记之后不能改回待处理。`,
          "",
        ) ?? "\0";
      // 取消对话框返回 null，那是「别记了」，不是「记一个空的」。
      if (reference === "\0") return;
    } else if (
      !confirm(`驳回 ${w.email} 的 ${usd(w.amount_cents)} 提现申请？这笔钱会退回他的可提现余额。`)
    ) {
      return;
    }
    try {
      await api.post(`/api/admin/withdrawals/${w.id}/status`, { status, reference });
      setNote({ text: `已${verb}。`, ok: true });
      await load();
    } catch (e) {
      setNote({ text: e instanceof Error ? e.message : `${verb}失败`, ok: false });
    }
  }

  async function save() {
    const p = Number(ratePct);
    const d = Number(days);
    if (!Number.isFinite(p) || p < 0 || p > 100) {
      setNote({ text: "返佣比例要在 0 到 100 之间。", ok: false });
      return;
    }
    if (!Number.isInteger(d) || d < 1 || d > 3650) {
      // 0 天等于第一笔付款到达时窗口已经过期 —— 一分钱也不会记，而且不报错。
      setNote({ text: "返佣期限要是 1 到 3650 之间的整数天。", ok: false });
      return;
    }
    const h = Number(holdDays);
    const m = Number(minPayout);
    if (!Number.isInteger(h) || h < 0 || h > 180) {
      setNote({ text: "冻结期要是 0 到 180 之间的整数天。", ok: false });
      return;
    }
    if (!Number.isFinite(m) || m <= 0) {
      setNote({ text: "提现门槛要大于 0。", ok: false });
      return;
    }
    setBusy(true);
    setNote(null);
    try {
      await api.put("/api/admin/referral/settings", {
        // 百分比转基点:30 → 3000。四舍五入,免得 12.345 存进去变成一个没人想要的数。
        rate_bps: Math.round(p * 100),
        window_days: d,
        enabled,
        auto_settle: autoSettle,
        hold_days: h,
        min_payout_cents: Math.round(m * 100),
        batch_enabled: batchEnabled,
      });
      setNote({ text: "已保存。只影响之后新绑定的推荐关系。", ok: true });
      await load();
    } catch (e) {
      setNote({ text: e instanceof Error ? e.message : "保存失败", ok: false });
    } finally {
      setBusy(false);
    }
  }

  async function settle(c: Commission, status: "settled" | "rejected") {
    const verb = status === "settled" ? "结算" : "驳回";
    if (!confirm(`${verb}给 ${c.referrer_email} 的 ${usd(c.commission_cents)}?`)) return;
    try {
      await api.post(`/api/admin/commissions/${c.id}/status`, { status });
      await load();
    } catch (e) {
      setNote({ text: e instanceof Error ? e.message : `${verb}失败`, ok: false });
    }
  }

  const head = HEADING[view];
  const pending = ledger?.rows.filter((r) => r.status === "pending") ?? [];

  /*
   * 失败时不要再挂着"加载中…"。
   *
   * 「还没回来」和「回来了但失败了」不是一回事,在红色错误提示旁边说加载中,谁也不
   * 知道该不该继续等。
   */
  const failed = (
    <EmptyState title="没能加载出来" hint="用上面的「重试」再试一次。" compact />
  );

  return (
    <div className="space-y-6">
      <PageHeader title={head.title} description={head.description} />

      {error && <ErrorState message={error} onRetry={() => void load()} />}

      {/*
        设置屏把这条消息放在保存按钮旁边,别的屏原本哪儿都不放 —— 于是「开通」按钮
        失败时把错误写进了 state,却没有任何一处渲染它,看上去就是点了没反应。
        一个动作要么看得见结果,要么根本不该有那个按钮。
      */}
      {note && view !== "commission" && (
        <div
          className={cn(
            "rounded-lg border px-4 py-2.5 text-sm",
            note.ok
              ? "border-emerald-200 bg-emerald-50 text-emerald-800 dark:border-emerald-900 dark:bg-emerald-950/40 dark:text-emerald-300"
              : "border-destructive/30 bg-destructive/10 text-destructive",
          )}
        >
          {note.text}
        </div>
      )}

      {/*
        设置到货之前**不画这个面板**。
        它每一个控件的初值都是 useState 里的字面量，而那不是"还没读到"，看起来就是
        "你当前的配置"。线上实测：比例 30%、期限 90 天、冻结 14 天、门槛 $50 这四个
        恰好和字面量一样，而 auto_settle 和 batch_enabled 两个布尔**是反的**
        （线上都是 true，字面量都是 false）。四个对、两个错，反而最难发现 ——
        运营会以为打款还是人工审核，而服务器那边 Stripe 已经在自动转账了。
        接口失败时同理：与其画一份编的配置，不如说"读不到"。
      */}
      {view === "commission" && !settings && !error && (
        <Panel className="mx-auto w-full max-w-2xl" bodyClassName="p-5" title="分销设置">
          <p className="text-sm text-muted-foreground">读取中…</p>
        </Panel>
      )}

      {view === "commission" && settings && (
        // 设置只有两个输入框,不该摊在 1280px 宽的面板里 —— 那样内容全贴在左边,
        // 右边三分之二是空的。面板自己收窄并居中,而不是面板全宽、里面的内容收窄。
        <Panel
          className="mx-auto w-full max-w-2xl"
          bodyClassName="p-5"
          title="规则"
          aside={
            settings && (
              <span className="whitespace-nowrap text-xs text-muted-foreground">
                {settings.referrals} 个推荐关系 · {settings.active} 个返佣中
              </span>
            )
          }
        >
          <div className="space-y-5">
            <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
              <div className="space-y-2">
                <Label htmlFor="rate">返佣比例</Label>
                <div className="relative">
                  <Input
                    id="rate"
                    className="h-11 pr-9 text-sm"
                    inputMode="decimal"
                    value={ratePct}
                    onChange={(e) => setRatePct(e.target.value)}
                  />
                  <span className="pointer-events-none absolute right-3.5 top-1/2 -translate-y-1/2 text-sm text-muted-foreground">
                    %
                  </span>
                </div>
                <p className="text-xs leading-relaxed text-muted-foreground">
                  被推荐人每次付款,按这个比例记给推荐人。
                </p>
              </div>

              <div className="space-y-2">
                <Label htmlFor="days">返佣期限</Label>
                <div className="relative">
                  <Input
                    id="days"
                    className="h-11 pr-9 text-sm"
                    inputMode="numeric"
                    value={days}
                    onChange={(e) => setDays(e.target.value)}
                  />
                  <span className="pointer-events-none absolute right-3.5 top-1/2 -translate-y-1/2 text-sm text-muted-foreground">
                    天
                  </span>
                </div>
                <p className="text-xs leading-relaxed text-muted-foreground">
                  {/* 别写死 90：旁边那个输入框就是这个值，改成 30 天之后这句话还说 90，
                      紧挨着输入框的一个数字对不上，会让人以为自己没改成功。 */}
                  从绑定那天算起，当前 {days || "—"} 天。
                </p>
              </div>
            </div>

            {/*
              * 打款。这一节和上面的「结算方式」是两件事：上面决定佣金**算不算数**，
              * 这里决定钱**什么时候真的出去**。
              */}
            <div className="space-y-3 rounded-lg border border-border p-4">
              <div className="flex flex-wrap items-start justify-between gap-3">
                <div className="min-w-0">
                  <Label>自动打款</Label>
                  <p className="mt-1 text-xs leading-relaxed text-muted-foreground">
                    过了冻结期、攒够门槛，系统自动转到对方的 Stripe 账户，不需要有人点确认。
                    关着的时候，用户自己在账号页提交提现申请。
                  </p>
                </div>
                <button
                  type="button"
                  role="switch"
                  aria-checked={batchEnabled}
                  onClick={() => setBatchEnabled((v) => !v)}
                  className={cn(
                    "relative h-6 w-11 shrink-0 rounded-full transition-colors",
                    batchEnabled ? "bg-primary" : "bg-muted",
                  )}
                >
                  <span
                    className={cn(
                      "absolute top-0.5 size-5 rounded-full bg-background transition-all",
                      batchEnabled ? "left-[1.375rem]" : "left-0.5",
                    )}
                  />
                </button>
              </div>

              {batchEnabled && (
                <p className="rounded-md bg-destructive/10 px-3 py-2 text-xs leading-relaxed text-destructive">
                  开启后服务器会在没有人操作的情况下向外转账。转账走 Stripe Connect，
                  只付给已经完成开户的推荐人；余额不足或账户未就绪时会自动跳过并在下一轮重试。
                </p>
              )}

              <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
                <div className="space-y-2">
                  <Label htmlFor="hold">冻结期</Label>
                  <div className="relative">
                    <Input
                      id="hold"
                      className="h-11 pr-9 text-sm"
                      inputMode="numeric"
                      value={holdDays}
                      onChange={(e) => setHoldDays(e.target.value)}
                    />
                    <span className="pointer-events-none absolute right-3.5 top-1/2 -translate-y-1/2 text-sm text-muted-foreground">
                      天
                    </span>
                  </div>
                  <p className="text-xs leading-relaxed text-muted-foreground">
                    佣金记下后要等这么久才允许打款。退款和拒付都是事后才发生的，
                    这段时间就是留给它们的。只影响之后新记的佣金。
                  </p>
                </div>

                <div className="space-y-2">
                  <Label htmlFor="minpay">提现门槛</Label>
                  <div className="relative">
                    <span className="pointer-events-none absolute left-3.5 top-1/2 -translate-y-1/2 text-sm text-muted-foreground">
                      $
                    </span>
                    <Input
                      id="minpay"
                      className="h-11 pl-7 text-sm"
                      inputMode="decimal"
                      value={minPayout}
                      onChange={(e) => setMinPayout(e.target.value)}
                    />
                  </div>
                  <p className="text-xs leading-relaxed text-muted-foreground">
                    同一个推荐人攒够这个数才发一笔。Stripe 每笔都有固定费用，
                    小额单独转很不划算。
                  </p>
                </div>
              </div>

              {settings && (settings.holding_cents > 0 || settings.ready_cents > 0) && (
                <p className="text-xs text-muted-foreground">
                  冻结中 <span className="font-medium text-foreground">{usd(settings.holding_cents)}</span>
                  {" · "}已到期待发 <span className="font-medium text-foreground">{usd(settings.ready_cents)}</span>
                </p>
              )}
            </div>

            <div className="space-y-2">
              <Label>结算方式</Label>
              {/*
                这一节决定的是「佣金算不算数」，不是「钱什么时候出去」—— 后者在上面那节。
                两张卡片而不是一个开关：差别是要不要一道人工复核，值得把后果写出来。
              */}
              <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
                {[
                  {
                    auto: false,
                    title: "人工审核",
                    body: "佣金先记成待结算，你逐笔审核通过后才进入冻结期。适合刚开始跑、还想看看每一笔是怎么来的。",
                  },
                  {
                    auto: true,
                    title: "自动通过",
                    body: "付款成功即记为已审核，不需要人点。仍然要过冻结期、攒够门槛才打款 —— 少的是人工复核这一步，不是安全垫。",
                  },
                ].map((opt) => (
                  <label
                    key={String(opt.auto)}
                    className={cn(
                      "flex cursor-pointer gap-2.5 rounded-lg border p-3.5 text-sm transition-colors",
                      autoSettle === opt.auto
                        ? "border-primary bg-secondary/60"
                        : "border-border hover:bg-muted/50",
                    )}
                  >
                    <input
                      type="radio"
                      name="settle-mode"
                      className="mt-0.5 size-4 shrink-0 accent-primary"
                      checked={autoSettle === opt.auto}
                      onChange={() => setAutoSettle(opt.auto)}
                    />
                    <span className="min-w-0">
                      {opt.title}
                      <span className="mt-0.5 block text-xs leading-relaxed text-muted-foreground">
                        {opt.body}
                      </span>
                    </span>
                  </label>
                ))}
              </div>
              {/* 切走的时候队列里还有人在等钱，就说一声 —— 那几笔不会自动发。 */}
              {autoSettle && (settings?.pending_withdrawals ?? 0) > 0 && (
                <p className="rounded-lg bg-amber-50 px-3 py-2 text-xs leading-relaxed text-amber-900 dark:bg-amber-950/40 dark:text-amber-200">
                  还有 {settings?.pending_withdrawals} 笔提现申请没处理。自动结算不会替你把它们发出去
                  —— 「提现申请」那一屏会一直留着，直到处理完为止。
                </p>
              )}
            </div>

            <label className="flex cursor-pointer items-start gap-2.5 rounded-lg border border-border p-3.5 text-sm transition-colors hover:bg-muted/50">
              <input
                type="checkbox"
                className="mt-0.5 size-4 shrink-0 accent-primary"
                checked={enabled}
                onChange={(e) => setEnabled(e.target.checked)}
              />
              <span className="min-w-0">
                开放推荐计划
                <span className="mt-0.5 block text-xs leading-relaxed text-muted-foreground">
                  关掉只是不再接受新的绑定。已经绑定的继续按原比例返佣到期满 ——
                  停掉一个计划不该毁掉已经许出去的承诺。
                </span>
              </span>
            </label>

            <div className="flex flex-wrap items-center gap-3 border-t border-border pt-4">
              <Button onClick={save} disabled={busy}>
                {busy && <Loader2 className="animate-spin" />}
                保存
              </Button>
              <span
                className={cn(
                  "min-w-0 flex-1 text-xs leading-relaxed",
                  note
                    ? note.ok
                      ? "text-emerald-600"
                      : "text-destructive"
                    : "text-muted-foreground",
                )}
              >
                {note?.text ?? "改动只影响之后新绑定的推荐关系,已绑定的比例和到期时间不变。"}
              </span>
            </div>
          </div>
        </Panel>
      )}

      {view === "commission-pending" && (
        <Panel
          title="待结算"
          aside={
            ledger && (
              <span className="whitespace-nowrap text-xs text-muted-foreground">
                待结算{" "}
                <span className="font-semibold text-foreground">
                  {usd(ledger.summary.pending_cents)}
                </span>
                {" · "}已结算 {usd(ledger.summary.settled_cents)}
              </span>
            )
          }
        >
          {error && !ledger ? (
            failed
          ) : !ledger ? (
            <TableSkeleton rows={5} columns={["24%", "24%", "12%", "8%", "12%"]} label="佣金读取中" />
          ) : pending.length === 0 ? (
            <EmptyState
              icon={Share2}
              title="没有待结算的佣金"
              hint="被推荐的用户在 Stripe 付款成功后,这里会自动出现一笔。"
              compact
            />
          ) : (
            <Table className="min-w-[58rem]">
              {/*
                六列，不是七。
                侧栏之后正文只有约 990px，七列要 1037px —— 于是「驳回」被推到横向滚动
                之外，一个必须点得到的动作按钮藏在了看不见的地方。日期是次要信息，挪进
                客户那一格当第二行，整张表就落回装得下的宽度。
              */}
              <TableHeader>
                <TableRow>
                  <TableHead className="w-[15rem]">推荐人</TableHead>
                  <TableHead className="w-[15rem]">付款客户</TableHead>
                  <TableHead numeric className="w-28">订单金额</TableHead>
                  <TableHead numeric className="w-20">比例</TableHead>
                  <TableHead numeric className="w-28">佣金</TableHead>
                  <TableHead className="w-52 text-right">操作</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {pending.map((c) => (
                  <TableRow key={c.id}>
                    <TableCell className="max-w-[15rem]">
                      <Truncate className="font-medium">{c.referrer_email}</Truncate>
                      {/* 只标手动的那几笔。绝大多数是付款时自动记入的，每行都写一遍
                          「自动记入」等于每行都没写 —— 值得标出来的是例外。 */}
                      {c.source !== "referral" && (
                        <div className="mt-0.5 text-xs text-muted-foreground">手动记入</div>
                      )}
                    </TableCell>
                    <TableCell className="max-w-[15rem]">
                      <Truncate className="text-muted-foreground">{c.customer_email}</Truncate>
                      <div className="mt-0.5 whitespace-nowrap text-xs text-muted-foreground">
                        {day(c.created_at)}
                      </div>
                    </TableCell>
                    <TableCell numeric className="text-muted-foreground">
                      {usd(c.amount_cents)}
                    </TableCell>
                    <TableCell numeric className="text-muted-foreground">{pct(c.rate_bps)}</TableCell>
                    <TableCell numeric className="font-semibold">{usd(c.commission_cents)}</TableCell>
                    <TableCell className="text-right">
                      <div className="flex justify-end gap-1.5">
                        <Button size="sm" variant="outline" onClick={() => void settle(c, "settled")}>
                          <Check /> 结算
                        </Button>
                        <Button size="sm" variant="ghost" onClick={() => void settle(c, "rejected")}>
                          <X /> 驳回
                        </Button>
                      </div>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </Panel>
      )}

      {view === "commission-referrers" && (
        <Panel
          title="账号"
          aside={
            <div className="flex items-center gap-3">
              {referrers && (
                <span className="whitespace-nowrap text-xs text-muted-foreground">
                  {referrers.granted} / {referrers.total} 已开通
                </span>
              )}
              <Input
                className="h-8 w-56 text-sm"
                placeholder="搜索邮箱"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
              />
            </div>
          }
        >
          {error && !referrers ? (
            failed
          ) : !referrers ? (
            <TableSkeleton rows={5} columns={["30%", "12%", "12%", "10%", "12%"]} label="账号读取中" />
          ) : referrers.rows.length === 0 ? (
            <EmptyState
              icon={UserPlus}
              title={query ? "没有匹配的账号" : "还没有账号"}
              hint={query ? "换个关键词试试。" : undefined}
              compact
            />
          ) : (
            <Table className="min-w-[56rem]">
              {/*
                列出全部账号,不只是推荐过人的 —— 开通资格就是在这一屏做的,还没开通的
                那些正是要找的人。已开通的排在最前面,后面按赚到多少排,列表很长而有
                意思的那一头很短。
              */}
              <TableHeader>
                <TableRow>
                  <TableHead className="w-[18rem]">账号</TableHead>
                  <TableHead className="w-32">邀请码</TableHead>
                  <TableHead numeric className="w-20">带来</TableHead>
                  <TableHead numeric className="w-20">返佣中</TableHead>
                  <TableHead numeric className="w-28">待结算</TableHead>
                  <TableHead numeric className="w-28">已结算</TableHead>
                  <TableHead className="w-32 text-right">推荐资格</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {referrers.rows.map((r) => (
                  <TableRow key={r.id} className={cn(!r.referral_enabled && "opacity-60")}>
                    <TableCell className="max-w-[18rem]">
                      <Truncate className="font-medium">{r.email}</Truncate>
                      {r.last_at && (
                        <div className="mt-0.5 whitespace-nowrap text-xs text-muted-foreground">
                          最近 {day(r.last_at)}
                        </div>
                      )}
                    </TableCell>
                    <TableCell>
                      {r.code ? (
                        <span className="rounded bg-secondary px-1.5 py-0.5 font-mono text-xs">
                          {r.code}
                        </span>
                      ) : (
                        <span className="text-muted-foreground">—</span>
                      )}
                    </TableCell>
                    <TableCell numeric>{r.invited || "—"}</TableCell>
                    <TableCell numeric className="text-muted-foreground">
                      {r.active || "—"}
                    </TableCell>
                    <TableCell numeric className={r.pending_cents ? "font-semibold" : "text-muted-foreground"}>
                      {usd(r.pending_cents)}
                    </TableCell>
                    <TableCell numeric className="text-muted-foreground">
                      {usd(r.settled_cents)}
                    </TableCell>
                    <TableCell className="text-right">
                      {r.referral_enabled ? (
                        <Button size="sm" variant="ghost" onClick={() => void grant(r, false)}>
                          <Check className="text-emerald-600" /> 已开通
                        </Button>
                      ) : (
                        <Button size="sm" variant="outline" onClick={() => void grant(r, true)}>
                          开通
                        </Button>
                      )}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}

          {referrers && (
            <Pager
              page={referrers.page}
              pages={referrers.pages}
              total={referrers.total}
              onPage={setReferrerPage}
            />
          )}
        </Panel>
      )}

      {view === "commission-referred" && (
        <Panel
          title="被推荐的账号"
          aside={
            referred && (
              <span className="whitespace-nowrap text-xs text-muted-foreground">
                {referred.length} 条
              </span>
            )
          }
        >
          {error && !referred ? (
            failed
          ) : !referred ? (
            <TableSkeleton rows={5} columns={["26%", "26%", "12%", "8%", "16%"]} label="推荐关系读取中" />
          ) : referred.length === 0 ? (
            <EmptyState
              icon={Users}
              title="还没有人用过邀请码"
              hint="用户在自己的账号页里能拿到邀请码和链接。"
              compact
            />
          ) : (
            <Table className="min-w-[58rem]">
              {/* 两个邮箱列各 15rem，不是 18rem —— 18 的时候整张表 1072px，正文只有
                  990px，最右边的「已产生」被切成一个 $。金额是这一屏的结论，不能是
                  唯一看不见的那一列。 */}
              <TableHeader>
                <TableRow>
                  <TableHead className="w-[15rem]">被推荐账号</TableHead>
                  <TableHead className="w-[15rem]">推荐人</TableHead>
                  <TableHead className="w-32">邀请码</TableHead>
                  <TableHead numeric className="w-20">比例</TableHead>
                  <TableHead className="w-44">返佣期</TableHead>
                  <TableHead numeric className="w-28">已产生</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {referred.map((r) => (
                  <TableRow key={`${r.code}-${r.referred_email}`}>
                    <TableCell className="max-w-[15rem]">
                      <Truncate className="font-medium">{r.referred_email}</Truncate>
                    </TableCell>
                    <TableCell className="max-w-[15rem]">
                      <Truncate className="text-muted-foreground">{r.referrer_email}</Truncate>
                    </TableCell>
                    <TableCell>
                      <span className="inline-flex items-center gap-1 rounded bg-secondary px-1.5 py-0.5 font-mono text-xs">
                        {/* 链接来的和手打邀请码来的是两件事,值得分开看。 */}
                        {r.source === "link" && <Link2 className="size-3 shrink-0" aria-label="链接" />}
                        {r.code}
                      </span>
                    </TableCell>
                    <TableCell numeric className="text-muted-foreground">{pct(r.rate_bps)}</TableCell>
                    <TableCell>
                      {r.active ? (
                        <Badge variant="success">返佣中</Badge>
                      ) : (
                        <Badge variant="outline">已结束</Badge>
                      )}
                      <div className="mt-0.5 whitespace-nowrap text-xs text-muted-foreground">
                        {day(r.created_at)} – {day(r.expires_at)}
                      </div>
                    </TableCell>
                    <TableCell numeric className={r.earned_cents > 0 ? "font-semibold" : "text-muted-foreground"}>
                      {usd(r.earned_cents)}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </Panel>
      )}

      {view === "commission-settlements" && (
        <Panel
          title="已结算"
          aside={
            settlements && (
              <span className="whitespace-nowrap text-xs text-muted-foreground">
                共 <span className="font-semibold text-foreground">{usd(settlements.total_cents)}</span>
                {" · "}自动 {settlements.auto_count} · 人工 {settlements.manual_count}
                {settlements.flagged > 0 && (
                  <span className="text-destructive">
                    {" · "}
                    {settlements.flagged} 笔待追回
                  </span>
                )}
              </span>
            )
          }
        >
          {error && !settlements ? (
            failed
          ) : !settlements ? (
            <TableSkeleton rows={5} columns={["24%", "24%", "12%", "12%", "14%"]} label="结算记录读取中" />
          ) : settlements.rows.length === 0 ? (
            <EmptyState
              icon={Wallet}
              title="还没有结算记录"
              hint="佣金结算之后会出现在这里 —— 自动结算的立刻出现，人工结算的在你点「结算」之后。"
              className="min-h-[26rem] justify-center"
            />
          ) : (
            <Table className="min-w-[58rem]">
              <TableHeader>
                <TableRow>
                  <TableHead className="w-[15rem]">推荐人</TableHead>
                  <TableHead className="w-[15rem]">付款客户</TableHead>
                  <TableHead numeric className="w-28">订单金额</TableHead>
                  <TableHead numeric className="w-20">比例</TableHead>
                  <TableHead numeric className="w-28">佣金</TableHead>
                  <TableHead className="w-[13rem]">结算方式</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {settlements.rows.map((r) => (
                  <TableRow key={r.id} className={cn(r.status === "reversed" && "opacity-60")}>
                    <TableCell className="max-w-[15rem]">
                      <Truncate className="font-medium">{r.referrer_email}</Truncate>
                    </TableCell>
                    <TableCell className="max-w-[15rem]">
                      <Truncate className="text-muted-foreground">{r.customer_email}</Truncate>
                    </TableCell>
                    <TableCell numeric className="text-muted-foreground">
                      {usd(r.amount_cents)}
                    </TableCell>
                    <TableCell numeric className="text-muted-foreground">{pct(r.rate_bps)}</TableCell>
                    <TableCell
                      numeric
                      className={cn(
                        "font-semibold",
                        r.status === "reversed" && "text-muted-foreground line-through",
                      )}
                    >
                      {usd(r.commission_cents)}
                    </TableCell>
                    <TableCell>
                      {/*
                       * 退款分两种，差别是钱有没有发出去，所以不能共用一个标签。
                       *   · reversed —— 结算了但还没付，直接撤销，不欠了。
                       *   · settled + reversed_at —— 钱已经出去了。软件收不回来，只能标出来
                       *     让人去处理（自动结算的那些就属于这一类：写进去的同时余额已经加了）。
                       */}
                      {r.status === "reversed" ? (
                        <Badge className="border-0 bg-secondary text-muted-foreground">已撤销</Badge>
                      ) : r.reversed_at ? (
                        <Badge className="border-0 bg-destructive/15 text-destructive">待追回</Badge>
                      ) : r.settled_by === "auto" ? (
                        <Badge className="border-0 bg-secondary text-muted-foreground">自动</Badge>
                      ) : r.settled_by ? (
                        <Badge className="border-0 bg-secondary text-muted-foreground">人工</Badge>
                      ) : (
                        /* 空的是这一列存在之前结算的老数据 —— 说「不详」比编一个来源诚实。 */
                        <Badge className="border-0 bg-muted text-muted-foreground">不详</Badge>
                      )}
                      <div className="mt-0.5 truncate whitespace-nowrap text-xs text-muted-foreground">
                        {r.reversed_at
                          ? `${day(r.reversed_at)} · ${
                              r.reversal_reason === "charge.dispute.created" ? "拒付" : "退款"
                            }`
                          : day(r.settled_at)}
                        {!r.reversed_at &&
                          r.settled_by &&
                          r.settled_by !== "auto" &&
                          ` · ${r.settled_by}`}
                      </div>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}

          {settlements && (
            <Pager
              page={settlements.page}
              pages={settlements.pages}
              total={settlements.total}
              onPage={setSettlementPage}
            />
          )}
        </Panel>
      )}

      {view === "commission-withdrawals" && (
        <Panel
          title="申请"
          aside={
            payouts && (
              <span className="whitespace-nowrap text-xs text-muted-foreground">
                待处理{" "}
                <span className="font-semibold text-foreground">
                  {usd(payouts.pending_total_cents)}
                </span>
              </span>
            )
          }
        >
          {error && !payouts ? (
            failed
          ) : !payouts ? (
            <TableSkeleton rows={5} columns={["24%", "12%", "24%", "10%", "14%"]} label="提现申请读取中" />
          ) : payouts.rows.length === 0 ? (
            /* 这一屏只有这一块内容，短短一条空状态摆在整页宽度里像没画完。 */
            <EmptyState
              icon={Wallet}
              title="还没有提现申请"
              hint="用户在自己账号页的「佣金 · 提现」里提交申请。"
              className="min-h-[26rem] justify-center"
            />
          ) : (
            <Table className="min-w-[60rem]">
              <TableHeader>
                <TableRow>
                  <TableHead className="w-[15rem]">用户</TableHead>
                  <TableHead numeric className="w-28">金额</TableHead>
                  <TableHead className="w-[17rem]">收款方式</TableHead>
                  <TableHead className="w-20">收款码</TableHead>
                  <TableHead className="w-24">状态</TableHead>
                  <TableHead className="w-44 text-right">操作</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {payouts.rows.map((w) => (
                  <TableRow key={w.id}>
                    <TableCell className="max-w-[15rem]">
                      {/* 名字在上、邮箱在下：转账时认的是人，对账时认的是邮箱，两个都要在。 */}
                      {w.name && <Truncate className="font-medium">{w.name}</Truncate>}
                      <Truncate className={cn("text-xs", w.name ? "text-muted-foreground" : "font-medium")}>
                        {w.email}
                      </Truncate>
                    </TableCell>
                    <TableCell numeric className="font-semibold">
                      {usd(w.amount_cents)}
                      <div className="text-xs font-normal text-muted-foreground">
                        已结算 {usd(w.settled_cents)}
                      </div>
                    </TableCell>
                    <TableCell className="max-w-[17rem]">
                      <div className="text-xs text-muted-foreground">
                        {METHOD_LABEL[w.method] ?? w.method}
                      </div>
                      <Truncate className="font-mono text-[13px]">{w.account}</Truncate>
                    </TableCell>
                    <TableCell>
                      {w.qr ? (
                        // 点开放大 —— 列表里的缩略图小到扫不了，而扫码就是它存在的理由。
                        <button
                          type="button"
                          onClick={() => setZoom(w.qr)}
                          className="block size-12 overflow-hidden rounded border border-border transition-opacity hover:opacity-80"
                          title="点击放大"
                        >
                          <img src={w.qr} alt="" className="size-full object-cover" />
                        </button>
                      ) : (
                        <span className="text-muted-foreground">—</span>
                      )}
                    </TableCell>
                    <TableCell>
                      <Badge className={cn("border-0", PAYOUT_STYLE[w.status])}>
                        {PAYOUT_LABEL[w.status] ?? w.status}
                      </Badge>
                      <div className="mt-0.5 whitespace-nowrap text-xs text-muted-foreground">
                        {day(w.created_at)}
                      </div>
                      {/*
                        * 「转账中」是这一屏最重要的一格：钱可能已经出去了，只是我们没收到回执。
                        * 这时手工再转一次就是同一笔钱付两遍，所以把原因和 tr_ 号都摆出来，
                        * 让人先去 Stripe 里按 metadata[withdrawal_id] 查一下。
                        */}
                      {w.status === "sending" && (
                        <div className="mt-1 max-w-[10rem] text-xs text-destructive">
                          先去 Stripe 核对，不要手工再付
                          {w.failure_reason && (
                            <span className="block text-muted-foreground">{w.failure_reason}</span>
                          )}
                        </div>
                      )}
                      {w.transfer_id && (
                        <div className="mt-0.5 truncate font-mono text-[11px] text-muted-foreground">
                          {w.transfer_id}
                        </div>
                      )}
                    </TableCell>
                    <TableCell className="text-right">
                      {w.status === "pending" ? (
                        <div className="flex justify-end gap-1.5">
                          <Button size="sm" variant="outline" onClick={() => void decide(w, "paid")}>
                            <Check /> 已支付
                          </Button>
                          <Button size="sm" variant="ghost" onClick={() => void decide(w, "rejected")}>
                            <X /> 驳回
                          </Button>
                        </div>
                      ) : (
                        // 处理完之后这一格就是这笔转账的凭据：什么时候、谁经的手、单号多少。
                        // 以前只有一个日期，三个月后有人问「到底发了没有」，答不上来。
                        <div className="text-xs text-muted-foreground">
                          <div>{w.paid_at ? day(w.paid_at) : "—"}</div>
                          {w.paid_by && <Truncate className="mt-0.5">{w.paid_by}</Truncate>}
                          {w.reference && (
                            <Truncate className="mt-0.5 font-mono" title={w.reference}>
                              {w.reference}
                            </Truncate>
                          )}
                        </div>
                      )}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}

          {payouts && (
            <Pager
              page={payouts.page}
              pages={payouts.pages}
              total={payouts.total}
              onPage={setPayoutPage}
            />
          )}
        </Panel>
      )}

      {/* 放大的收款码。点背景任意处关掉 —— 这一层只是为了看清楚，不值得一个对话框。 */}
      {zoom && (
        <div
          role="button"
          tabIndex={0}
          onClick={() => setZoom(null)}
          onKeyDown={(e) => e.key === "Escape" && setZoom(null)}
          className="fixed inset-0 z-50 grid place-items-center bg-black/60 p-8"
        >
          <img src={zoom} alt="" className="max-h-[80vh] max-w-[80vw] rounded-lg bg-white p-3" />
        </div>
      )}
    </div>
  );
}
