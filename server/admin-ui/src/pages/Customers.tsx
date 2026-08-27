import { useCallback, useEffect, useRef, useState } from "react";
import { EmptyState } from "@/components/EmptyState";
import { ErrorState } from "@/components/ErrorState";
import { PageHeader } from "@/components/PageHeader";
import { Pager, PAGE_SIZE } from "@/components/Pager";
import { Toolbar } from "@/components/Toolbar";
import { Stat } from "@/components/Stat";
import { TableSkeleton } from "@/components/TableSkeleton";
import { SectionReveal } from "@/components/motion/section-reveal";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select } from "@/components/ui/select";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow, Truncate } from "@/components/ui/table";
import { api } from "@/lib/api";
import {
  creditCentsFromRaw,
  creditDenominator,
  rawCentsFromCreditDollars,
  settingsLoaded,
  planKeys,
  useSettings,
  memberTierSupported,
} from "@/lib/settings";
import { cn } from "@/lib/utils";
import { cents, num, when } from "@/lib/format";

/**
 * 客户 —— 运营的主力屏。一天里真正会做的四件事：查一个人现在能不能用、给刚付钱的人加额度、
 * 改/续/退套餐、以及处理角色和注销。
 *
 * 相对旧后台（static/admin.html 第 681 行起）补上的三处：
 *  1. 表格加了「时段用量 / 上限」和「最近登录」。GET /api/admin/users 一直在返回
 *     quota_window_cents / quota_window_cap_cents / last_login_at（auth.rs:25-37），旧表一个都没显示，
 *     于是「他为什么用不了」只能靠猜。这两列正是运营点开一个客户的两个理由。
 *  2. 接上了 POST /api/admin/users/:id/grant（codes.rs:363）—— 全站原本没有任何入口。它是**叠加**充值，
 *     「客户付了 $20」不该再要求人脑算出一个新的绝对余额。
 *  3. 绝对改写（POST .../credits）保留，但和叠加充值分成两块、各自写清楚动词，不会点错。
 *
 * 刻意没有搬过来的旧东西：
 *  - 每行五颗按钮（角色/编辑/取消会员/删除）—— 全部收进弹窗，行里只留「管理」。
 *  - window.confirm() 弹三次「确定吗」—— 改成就地二次确认，破坏性操作看得见对象。
 *  - 改完之后那段黄色高亮的 WebSocket 单行热补丁（admin.html:patchUserRow）—— 每次操作后直接重拉
 *     500 行，逻辑少一半，也不会出现半新半旧的行。
 *  - 「技能」侧栏那三块统计（最长指令 / 空指令 / 平均长度）—— 与客户运营无关，属于别的屏。
 */

/** codes.rs:12 —— 合法套餐；"none" 由 admin_set_plan / cancel-plan 当作退订处理。 */
const PLANS = ["trial", "basic", "pro", "power", "ultra"] as const;

/**
 * 余额的面值换算：N 个真实计费分 = 客户看到的 $1.00 额度（默认 663）。
 * 运营填的、客户看的都是这个面值；写进 credits_cents 的是真实分。
 *
 * 分母不再写死在这里 —— 它是服务端 app_settings 的值，由 lib/settings.ts 下发。
 * 之前这个数在四个文件里各有一份副本（这里、Billing.tsx、static/admin.html、
 * ide/src/main.js），其中三份在写路径上，改一处不改其余就会「发出去多少」和
 * 「显示多少」当场对不上，且不报错。
 *
 * quota_* 与 credits_cents **是同一个单位**（真实计费分），所以一样要走这层换算：
 * 结算时 split_fused_charge 把同一笔 requested_cost 拆成 quota_cents 和 wallet_cents，
 * 分别扣在 quota_total_cents 和 credits_cents 上（models.rs 的结算 UPDATE）。
 *
 * 这层换算是有损的：1 个面值分 ≈ 6.63 个真实分，raw → 两位小数 → raw 不一定回到原值
 * （1000 → "1.51" → 1001）。所以「改写余额」只在输入框真的被改过时才可提交，
 * 见下面的 balanceDirty —— 否则光是开弹窗再点一下保存，就会无声地动一次账。
 */
const creditCents = creditCentsFromRaw;
const creditInput = (raw?: number) => ((raw || 0) / creditDenominator()).toFixed(2);
const toRawCents = rawCentsFromCreditDollars;
const MILLI = 1000;

type User = {
  id: string;
  email: string;
  role?: string;
  plan?: string;
  plan_expires_at?: string | null;
  credits_cents?: number;
  free_points?: number;
  quota_total_cents?: number;
  quota_window_cap_cents?: number;
  quota_window_cents?: number;
  quota_window_reset_at?: string | null;
  quota_weekly_cap_cents?: number;
  quota_week_used_cents?: number;
  quota_week_reset_at?: string | null;
  created_at?: string;
  last_login_at?: string | null;
};

/** codes.rs:288 user_summary() —— 四个写操作统一回这三个字段，用它把弹窗里的输入框拨回真值。 */
type UserSummary = {
  plan?: string;
  plan_expires_at?: string | null;
  credits_cents?: number;
};
type WriteResp = { ok?: boolean; user?: UserSummary };

/** 四个统计卡片算的是**全量**，不是当页也不是筛选后的——服务端一条独立聚合出的。 */
type Stats = { total: number; members: number; drained: number; recent: number };
type Paged = { users?: User[]; total?: number; page?: number; page_size?: number; stats?: Stats };


const isActive = (u: User) =>
  !!u.plan && u.plan !== "none" && (!u.plan_expires_at || new Date(u.plan_expires_at).getTime() > Date.now());

/** 存的是「本时段还剩多少」（models.rs:3736 扣费时递减），用量要拿上限减出来。 */
function windowUse(u: User) {
  const cap = u.quota_window_cap_cents || 0;
  if (cap <= 0) return null;
  const left = Math.max(0, Math.min(cap, u.quota_window_cents ?? cap));
  return { cap, left, used: cap - left, pct: Math.round(((cap - left) / cap) * 100) };
}

/**
 * format.ts 的 when() 只处理过去：它算 (now - t)，未来时间会得到负数天并渲染成「-29 天前」。
 * 到期时间天然在未来，所以这里单独算剩余。正确的长期修法是给 when() 补一个未来分支 ——
 * 那是 format.ts 的事，一个页面不该顺手改掉全站的时间口径。
 */
function untilExpiry(iso?: string | null) {
  if (!iso) return "永久";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "—";
  const ms = d.getTime() - Date.now();
  if (ms <= 0) return "已过期";
  const days = Math.floor(ms / 86_400_000);
  if (days === 0) return "今天到期";
  if (days === 1) return "明天到期";
  return `还有 ${days} 天`;
}

/** ISO ⇄ <input type="datetime-local"> 的值。只是控件的取值格式，不是给人看的时间。 */
function toLocalInput(iso?: string | null) {
  if (!iso) return "";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "";
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}T${p(d.getHours())}:${p(d.getMinutes())}`;
}

export function Customers() {
  // 订阅面值分母：设置到货后金额要重算一次，否则表格会停在兜底面值上。
  useSettings();
  const [users, setUsers] = useState<User[]>([]);
  const [total, setTotal] = useState(0);
  const [stats, setStats] = useState<Stats>({ total: 0, members: 0, drained: 0, recent: 0 });
  const [page, setPage] = useState(1);
  const [meId, setMeId] = useState("");
  const [q, setQ] = useState("");
  // 输入框里的字和真正发出去的查询是两回事：每敲一个字母打一次数据库没必要，
  // 而且回包乱序时列表会闪。300ms 停下来再查。
  const [debouncedQ, setDebouncedQ] = useState("");
  const [planFilter, setPlanFilter] = useState("");
  const [loading, setLoading] = useState(true);
  const [err, setErr] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  // 轮询要知道弹窗开着没，但不该因此重建定时器（会把 30 秒的节拍重置成每次开关弹窗一次）。
  // 写在 effect 里而不是渲染期间：渲染期间写 ref 在 StrictMode / 并发渲染下是未定义行为。
  const openRef = useRef(false);
  useEffect(() => {
    openRef.current = !!editingId;
  }, [editingId]);

  const load = useCallback(async () => {
    const qs = new URLSearchParams({ page: String(page), page_size: String(PAGE_SIZE) });
    if (debouncedQ) qs.set("q", debouncedQ);
    if (planFilter) qs.set("filter", planFilter);
    const r = await api.get<Paged>(`/api/admin/customers?${qs}`);
    setUsers(Array.isArray(r?.users) ? r.users : []);
    setTotal(Number(r?.total) || 0);
    if (r?.stats) setStats(r.stats);
    // 服务端会把越界的页码夹回最后一页（筛窄了、或者别人刚删了人）。它回的才是真实页码，
    // 不同步回来的话分页控件会一直停在一个不存在的页上，按钮亮着却没反应。
    if (r?.page && r.page !== page) setPage(r.page);
    setErr("");
  }, [page, debouncedQ, planFilter]);

  useEffect(() => {
    const t = setTimeout(() => setDebouncedQ(q.trim()), 300);
    return () => clearTimeout(t);
  }, [q]);

  // 换了搜索词或筛选项就回第一页。留在第 5 页上是「明明搜到了却是空的」最常见的来源。
  useEffect(() => {
    setPage(1);
  }, [debouncedQ, planFilter]);

  useEffect(() => {
    let alive = true;
    const tick = async () => {
      try {
        await load();
      } catch (e) {
        if (alive) setErr(e instanceof Error ? e.message : "加载失败");
      } finally {
        if (alive) setLoading(false);
      }
    };
    tick();
    // 时段额度是这一屏的看点，让它自己走；弹窗开着时不刷，免得脚下的数字乱跳。
    const t = setInterval(() => { if (!openRef.current) tick(); }, 30_000);
    return () => { alive = false; clearInterval(t); };
  }, [load]);

  useEffect(() => {
    let alive = true;
    // 自己的 id：删号和改角色都不允许作用于自己（auth.rs:695 / auth.rs:718），
    // 与其等服务端报错，不如提前把按钮关掉并说明原因。只跟登录态有关，不跟着翻页重取。
    api.get<{ id?: string }>("/api/me").then((m) => { if (alive) setMeId(m?.id || ""); }).catch(() => {});
    return () => { alive = false; };
  }, []);

  // 这一页的行。筛选、搜索、统计全在服务端做了 —— 客户端手里只有当页那 20 行，
  // 在这 20 行里再筛一遍不是筛选，是骗人。
  const list = users;
  const pages = Math.max(1, Math.ceil(total / PAGE_SIZE));

  const editing = editingId ? users.find((u) => u.id === editingId) : undefined;

  return (
    <div className="space-y-6">
      <PageHeader title="客户" description="谁现在还能用、谁付了钱、谁该退订。改动立即生效。" />

      <ErrorState message={err} onRetry={() => load().catch(() => {})} />

      {/* 入场错峰：标题 0，往下每段 +70ms（展示站 SectionReveal 的 Math.min(i,4)*70）。 */}
      <SectionReveal as="section" delay={70} className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <Stat label="客户总数" value={num(stats.total)} />
        <Stat label="有效会员" value={num(stats.members)} />
        <Stat
          label="时段额度用尽"
          value={num(stats.drained)}
          hint={stats.drained ? "这些人现在发不出请求" : "没有人被卡住"}
        />
        <Stat label="7 天内登录" value={num(stats.recent)} />
      </SectionReveal>

      <SectionReveal as="section" delay={140} className="space-y-4">
      <Toolbar
        query={q}
        onQuery={setQ}
        placeholder="搜索邮箱 / 角色 / 套餐 / ID…"
        count={q.trim() || planFilter ? `${num(total)} / ${num(stats.total)}` : `${num(total)} 位`}
        onRefresh={() => load().catch(() => {})}
      >
        <Select
          className="w-44"
          value={planFilter}
          onChange={(e) => setPlanFilter(e.target.value)}
          aria-label="筛选客户"
        >
          <option value="">全部客户</option>
          <option value="member">有效会员</option>
          <option value="none">无会员</option>
          <option value="admin">管理员</option>
          {PLANS.map((p) => (
            <option key={p} value={p}>套餐：{p}</option>
          ))}
        </Select>
      </Toolbar>

      <div className="overflow-hidden rounded-xl border border-border bg-card">
        {loading && !users.length ? (
          <TableSkeleton
            rows={6}
            columns={["22%", "7%", "9%", "17%", "9%", "9%"]}
            label="客户列表读取中"
          />
        ) : (
          <>
        {/*
          列宽写死，不让浏览器按内容分配：邮箱长度从 8 到 80 都有，交给 auto layout 的结果是
          每翻一页列宽都在动。68rem 是七列都放得下的下限，比它窄就横向滚动 —— 挤成竖排的
          「已/用/完」不是密度，是不能用。
        */}
        <Table className="min-w-[68rem]">
          <TableHeader>
            <TableRow>
              <TableHead className="w-[22rem]">账号</TableHead>
              <TableHead className="w-28">角色</TableHead>
              <TableHead className="w-32">套餐</TableHead>
              <TableHead className="w-48">本时段用量</TableHead>
              <TableHead numeric className="w-28">余额</TableHead>
              <TableHead className="w-32">最近登录</TableHead>
              <TableHead className="w-24 text-right" />
            </TableRow>
          </TableHeader>
          <TableBody>
            {list.map((u) => {
              const w = windowUse(u);
              const active = isActive(u);
              return (
                <TableRow key={u.id}>
                  <TableCell className="max-w-[22rem]">
                    <Truncate className="font-medium">{u.email}</Truncate>
                    <div className="mt-0.5 whitespace-nowrap text-xs text-muted-foreground">
                      注册 {when(u.created_at)}
                    </div>
                  </TableCell>
                  <TableCell>
                    {u.role === "admin" ? <Badge>管理员</Badge> : <Badge variant="outline">用户</Badge>}
                    {u.id === meId && <div className="mt-1 text-xs text-muted-foreground">当前登录</div>}
                  </TableCell>
                  <TableCell>
                    {!u.plan || u.plan === "none" ? (
                      <span className="text-muted-foreground">—</span>
                    ) : active ? (
                      <>
                        <Badge variant="success">{u.plan}</Badge>
                        <div className="mt-1 text-xs text-muted-foreground">
                          {untilExpiry(u.plan_expires_at)}
                        </div>
                      </>
                    ) : (
                      <>
                        <Badge variant="outline">{u.plan}</Badge>
                        <div className="mt-1 text-xs text-muted-foreground">已过期</div>
                      </>
                    )}
                  </TableCell>
                  <TableCell>
                    {!w ? (
                      <span className="text-muted-foreground">—</span>
                    ) : (
                      <div className="w-44">
                        <div className="flex items-center gap-1.5 whitespace-nowrap text-xs tabular-nums">
                          <span className="font-medium">{cents(creditCents(w.used))}</span>
                          <span className="text-muted-foreground">/ {cents(creditCents(w.cap))}</span>
                          {w.left <= 0 && (
                            <Badge variant="outline" className="border-destructive/40 text-destructive">
                              已用完
                            </Badge>
                          )}
                        </div>
                        <div className="mt-1.5 h-1 w-full overflow-hidden rounded-full bg-secondary">
                          <div className="h-full rounded-full bg-primary" style={{ width: `${w.pct}%` }} />
                        </div>
                      </div>
                    )}
                  </TableCell>
                  <TableCell numeric>{cents(creditCents(u.credits_cents))}</TableCell>
                  <TableCell className="whitespace-nowrap text-muted-foreground">
                    {u.last_login_at ? when(u.last_login_at) : "从未登录"}
                  </TableCell>
                  <TableCell className="text-right">
                    <Button variant="outline" size="sm" onClick={() => setEditingId(u.id)}>管理</Button>
                  </TableCell>
                </TableRow>
              );
            })}
          </TableBody>
        </Table>
        {!list.length && (
          <EmptyState
            title={stats.total ? "没有匹配的客户" : "暂无客户"}
            hint={
              stats.total
                ? "关键词匹配邮箱 / 角色 / 套餐 / ID，筛选和它是「且」的关系。"
                : "有人注册后会出现在这里。"
            }
            action={
              stats.total ? (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => {
                    setQ("");
                    setPlanFilter("");
                  }}
                >
                  清空筛选
                </Button>
              ) : undefined
            }
          />
        )}
        <Pager page={page} pages={pages} total={total} unit="位" onPage={setPage} />
          </>
        )}
      </div>
      </SectionReveal>

      <Dialog open={!!editing} onOpenChange={(o) => { if (!o) setEditingId(null); }}>
        {editing && (
          <CustomerDialog
            key={editing.id}
            user={editing}
            isSelf={editing.id === meId}
            reload={load}
            onClose={() => setEditingId(null)}
          />
        )}
      </Dialog>
    </div>
  );
}

/* ------------------------------------------------------------------ */

function Fact({ label, value, hint }: { label: string; value: string; hint?: string }) {
  return (
    <div>
      <div className="type-eyebrow">{label}</div>
      <div className="mt-1 text-sm font-medium tabular-nums">{value}</div>
      {hint && <div className="text-xs text-muted-foreground">{hint}</div>}
    </div>
  );
}

function CustomerDialog({
  user, isSelf, reload, onClose,
}: {
  user: User; isSelf: boolean; reload: () => Promise<void>; onClose: () => void;
}) {
  const [topUp, setTopUp] = useState("");
  const [balanceMode, setBalanceMode] = useState<"add" | "set">("add");
  const [balance, setBalance] = useState(creditInput(user.credits_cents));
  const [plan, setPlan] = useState(user.plan || "none");
  const [expiry, setExpiry] = useState(toLocalInput(user.plan_expires_at));
  const [resetQuotas, setResetQuotas] = useState(true);
  const [extendPlan, setExtendPlan] = useState<string>(user.plan && user.plan !== "none" ? user.plan : "basic");
  const [extendDays, setExtendDays] = useState("30");
  const [confirming, setConfirming] = useState<"cancel" | "delete" | "unsubscribe" | null>(null);
  const [busy, setBusy] = useState("");
  const [msg, setMsg] = useState<{ text: string; ok: boolean } | null>(null);

  const settings = useSettings();
  const w = windowUse(user);
  const weeklyCap = user.quota_weekly_cap_cents || 0;

  /**
   * 每个写操作成功后都要把输入框拨回服务端真值，否则「先叠加开通、再保存套餐」会拿弹窗里
   * 那份过期的到期时间把刚顺延的天数写回去（apply_plan 会改 plan 和 plan_expires_at，
   * codes.rs:239-246），退订之后不重置同理 —— 一点「保存套餐」就把刚取消的会员连额度一起复活。
   */
  const syncPlanFields = (s?: UserSummary) => {
    if (!s) return;
    setPlan(s.plan || "none");
    setExpiry(toLocalInput(s.plan_expires_at));
  };
  const syncBalanceField = (s?: UserSummary) => {
    if (!s || s.credits_cents == null) return;
    setBalance(creditInput(s.credits_cents));
  };

  async function act<T>(
    key: string,
    fn: () => Promise<T>,
    done: string,
  ): Promise<{ ok: true; data: T } | { ok: false }> {
    setBusy(key);
    setMsg(null);
    try {
      const data = await fn();
      await reload();
      setMsg({ text: done, ok: true });
      setConfirming(null);
      return { ok: true, data };
    } catch (e) {
      setMsg({ text: e instanceof Error ? e.message : "操作失败", ok: false });
      return { ok: false };
    } finally {
      setBusy("");
    }
  }

  const grantCredits = async () => {
    const r = await act(
      "grant",
      () => api.post<WriteResp>(`/api/admin/users/${user.id}/grant`, {
        kind: "credits",
        credits_cents: toRawCents(topUp),
      }),
      `已充值 ${cents(Math.round((Number.parseFloat(topUp) || 0) * 100))}`,
    );
    // 失败时留住输入框，运营改一个数就能重试；成功才清空。
    if (r.ok) {
      setTopUp("");
      syncBalanceField(r.data.user);
    }
  };

  const setAbsoluteBalance = async () => {
    const r = await act(
      "credits",
      () => api.post<WriteResp>(`/api/admin/users/${user.id}/credits`, {
        credits_cents: toRawCents(balance),
      }),
      "余额已改写",
    );
    if (r.ok) syncBalanceField(r.data.user);
  };

  const savePlan = async () => {
    const r = await act(
      "plan",
      () => api.post<WriteResp>(`/api/admin/users/${user.id}/plan`, {
        plan,
        expires_at: expiry ? new Date(expiry).toISOString() : null,
        reset_quotas: resetQuotas,
      }),
      plan === "none" ? "已退订" : "套餐已保存",
    );
    if (r.ok) syncPlanFields(r.data.user);
  };

  const grantPlan = async () => {
    const r = await act(
      "extend",
      () => api.post<WriteResp>(`/api/admin/users/${user.id}/grant`, {
        kind: "plan",
        plan: extendPlan,
        duration_days: Number.parseInt(extendDays, 10) || 0,
      }),
      `已叠加 ${extendPlan} ${extendDays} 天`,
    );
    if (r.ok) syncPlanFields(r.data.user);
  };

  const cancelPlan = async () => {
    const r = await act(
      "cancel",
      () => api.post<WriteResp>(`/api/admin/users/${user.id}/cancel-plan`, {}),
      "已取消会员",
    );
    if (r.ok) syncPlanFields(r.data.user);
  };

  const topUpValue = Number.parseFloat(topUp) || 0;
  const extendValue = Number.parseInt(extendDays, 10) || 0;
  // grant 是纯加法（codes.rs:279 credits_cents + $1），服务端不拦负数结果 ——
  // 只有 set_credits 拒绝负数。别让「扣减」把钱包扣穿。
  const topUpResultRaw = (user.credits_cents || 0) + toRawCents(topUp);
  const topUpOverdraft = topUpValue !== 0 && topUpResultRaw < 0;
  const balanceValue = Number.parseFloat(balance);
  // 面值 → raw 是有损的，原样保存会让余额漂 1 个 raw 分。没改过就不许提交。
  const balanceDirty = balance !== creditInput(user.credits_cents);

  return (
    <DialogContent className="flex max-h-[85vh] max-w-2xl flex-col overflow-y-auto">
      <DialogHeader>
        <DialogTitle className="truncate">{user.email}</DialogTitle>
        <DialogDescription>
          {user.role === "admin" ? "管理员" : "普通用户"} · ID {user.id.slice(0, 8)}
          {isSelf && " · 这是你自己的账号"}
        </DialogDescription>
      </DialogHeader>

      <div className="grid grid-cols-2 gap-x-6 gap-y-4 rounded-xl border border-border bg-secondary/40 p-4 sm:grid-cols-4">
        <Fact
          label="本时段"
          value={w ? `${cents(creditCents(w.used))} / ${cents(creditCents(w.cap))}` : "—"}
          hint={w ? (w.left <= 0 ? "已用完，等待刷新" : `剩 ${cents(creditCents(w.left))} · 每 5.5 小时刷新`) : "无会员额度"}
        />
        <Fact
          label="本周"
          // 用量已折面值，上限必须一起折 —— 否则同一个 "已用 / 上限" 里两个数差 6.63 倍。
          value={weeklyCap > 0 ? `${cents(creditCents(user.quota_week_used_cents))} / ${cents(creditCents(weeklyCap))}` : cents(creditCents(user.quota_week_used_cents))}
          hint={weeklyCap > 0 ? `真实计费上限 ${cents(weeklyCap)}` : "无周上限（0 = 不限）"}
        />
        <Fact label="总额度余量" value={cents(creditCents(user.quota_total_cents))} />
        <Fact
          label="钱包余额"
          value={cents(creditCents(user.credits_cents))}
          hint={`真实计费 ${cents(user.credits_cents)}`}
        />
        <Fact
          label="免费点数"
          value={`${num(Math.round((user.free_points ?? 0) / MILLI))} 点`}
          hint={
            /*
              每日赠送分了两档（设置 → 每日赠送），这里必须跟着分：不然打开一个会员的详情，
              看到的是非会员那一档的数字，和他池子里实际那份对不上，而且不会有任何报错 ——
              而这正是运营查「他怎么才这么点」时看的那一格。
              会员判据用本文件顶上已有的 isActive()，**不另写一份**（服务端那侧是 auth.rs 的
              ACTIVE_MEMBER_SQL，那条常量的注释点名了这里是它的前端镜像）。
              旧网关不下发第二档：拿不到就照旧显示单档、也不标注是哪一档，不编一个数出来。
            */
            !memberTierSupported(settings)
              ? `每日 ${settings.free_points_daily} 点`
              : isActive(user)
                ? `每日 ${settings.free_points_daily_member_effective ?? settings.free_points_daily} 点（会员档）`
                : `每日 ${settings.free_points_daily} 点（非会员档）`
          }
        />
        <Fact
          label="套餐"
          value={user.plan && user.plan !== "none" ? user.plan : "无"}
          hint={user.plan && user.plan !== "none" ? untilExpiry(user.plan_expires_at) : undefined}
        />
        <Fact label="最近登录" value={user.last_login_at ? when(user.last_login_at) : "从未登录"} hint={`注册 ${when(user.created_at)}`} />
      </div>

      {msg && (
        <p role="alert" className={msg.ok ? "text-sm text-success" : "text-sm text-destructive"}>{msg.text}</p>
      )}

      {/* Four stacked sections in one long scroll made this dialog a wall. They are three
          different jobs — money, membership, the account itself — so they are three tabs.
          Nothing about the logic changed; only what you have to scroll past to reach it. */}
      <Tabs defaultValue="credit" className="mt-2">
        <TabsList className="w-full">
          <TabsTrigger value="credit" className="flex-1">额度</TabsTrigger>
          <TabsTrigger value="plan" className="flex-1">套餐</TabsTrigger>
          <TabsTrigger value="account" className="flex-1">账号</TabsTrigger>
        </TabsList>

        <TabsContent value="credit" className="space-y-6 pt-4">
        {/* One balance field, two verbs. These were two full sections — heading, hint, label,
            input, buttons, twice — for the same underlying number, which is why the dialog felt
            padded out. The mode switch makes the DIFFERENCE the visible thing (加 vs 设为), which
            is the part that actually matters and the part that was easiest to get wrong.
            Both handlers and both guards are unchanged. */}
        <section>
          <div className="flex flex-wrap items-center justify-between gap-3">
            <h3 className="text-sm font-semibold">余额</h3>
            <div
              role="radiogroup" aria-label="余额操作方式"
              className="inline-flex h-9 items-center rounded-full border border-border bg-card p-1"
            >
              {([["add", "增加余额"], ["set", "调整余额"]] as const).map(([m, t]) => (
                <button
                  key={m} type="button" role="radio" aria-checked={balanceMode === m}
                  onClick={() => setBalanceMode(m)}
                  className={cn(
                    "rounded-full px-3 py-1 text-xs font-medium transition-colors",
                    balanceMode === m
                      ? "bg-foreground text-background"
                      : "text-muted-foreground hover:text-foreground",
                  )}
                >
                  {t}
                </button>
              ))}
            </div>
          </div>

          <p className="mt-1.5 text-center text-xs text-muted-foreground">
            {balanceMode === "add"
              ? "客户付款后用这个。填多少加多少，不必先算总额；扣减填负数。"
              : "对账修正才用。这是覆盖不是增加，不能为负，没改动时不可提交。"}
          </p>

          <div className="mt-3 flex flex-wrap items-end justify-center gap-2">
            <div className="w-40">
              <Label htmlFor="balance-field">
                {balanceMode === "add" ? "增加（$）" : "余额（$）"}
              </Label>
              {balanceMode === "add" ? (
                <Input
                  id="balance-field" type="number" step="0.01" value={topUp}
                  onChange={(e) => setTopUp(e.target.value)} placeholder="20.00"
                />
              ) : (
                <Input
                  id="balance-field" type="number" step="0.01" min="0" value={balance}
                  onChange={(e) => setBalance(e.target.value)}
                />
              )}
            </div>

            {balanceMode === "add" && (
              <div className="flex gap-1 pb-0.5">
                {["10", "20", "50"].map((v) => (
                  <Button key={v} type="button" variant="ghost" size="sm" onClick={() => setTopUp(v)}>
                    +${v}
                  </Button>
                ))}
              </div>
            )}

            {balanceMode === "add" ? (
              <Button
                size="sm"
                disabled={busy !== "" || topUpValue === 0 || topUpOverdraft}
                onClick={grantCredits}
              >
                {busy === "grant" ? "充值中…" : "充值"}
              </Button>
            ) : (
              <Button
                variant="outline" size="sm"
                disabled={busy !== "" || !balanceDirty || !Number.isFinite(balanceValue) || balanceValue < 0}
                onClick={setAbsoluteBalance}
              >
                {busy === "credits" ? "保存中…" : "改写余额"}
              </Button>
            )}

            <span className="pb-2.5 text-xs tabular-nums text-muted-foreground">
              当前 {cents(creditCents(user.credits_cents))}
            </span>
          </div>

          {balanceMode === "add" && topUpValue !== 0 && (
            <p
              className={cn(
                "mt-2 text-center text-xs tabular-nums",
                topUpOverdraft ? "text-destructive" : "text-muted-foreground",
              )}
            >
              {topUpOverdraft
                ? `扣减后余额会变成负数（${cents(creditCents(topUpResultRaw))}），请改小扣减额`
                : `充值后余额 ${cents(creditCents(topUpResultRaw))}`}
            </p>
          )}
        </section>
        </TabsContent>

        <TabsContent value="plan" className="pt-4">
        <section>
          <h3 className="text-sm font-semibold">套餐</h3>
          <p className="mt-1 text-xs text-muted-foreground">
            上面一行是「改成」，下面一行是「再加」。选 none 保存等同退订，会要求二次确认。
          </p>

          <div className="mt-3 grid gap-3 sm:grid-cols-[minmax(0,10rem)_minmax(0,1fr)_auto] sm:items-end">
            <div>
              <Label htmlFor="plan">改成</Label>
              <Select
                id="plan"
                value={plan}
                onChange={(e) => {
                  setPlan(e.target.value);
                  if (confirming === "unsubscribe") setConfirming(null);
                }}
              >
                <option value="none">none（退订）</option>
                {PLANS.map((p) => <option key={p} value={p}>{p}</option>)}
              </Select>
            </div>
            <div>
              <Label htmlFor="expiry">到期时间</Label>
              <Input
                id="expiry" type="datetime-local" value={expiry}
                onChange={(e) => setExpiry(e.target.value)} disabled={plan === "none"}
              />
            </div>
            {/*
              plan="none" 走的是和 cancel-plan 完全相同的一条 SQL（codes.rs:456-467 vs 519-525）：
              套餐、到期、时段/周/总额度一起清零。既然「取消会员」要二次确认，这条路不能一点就炸。
            */}
            <Button
              size="sm"
              disabled={busy !== "" || confirming === "unsubscribe"}
              onClick={() => {
                if (plan === "none") { setMsg(null); setConfirming("unsubscribe"); return; }
                savePlan();
              }}
            >
              {busy === "plan" ? "保存中…" : plan === "none" ? "退订…" : "保存套餐"}
            </Button>
          </div>

          {confirming === "unsubscribe" && (
            <div className="mt-3 flex flex-wrap items-center gap-2 rounded-lg border border-destructive/40 bg-destructive/5 p-3">
              <span className="text-sm text-muted-foreground">
                保存 none 会立即清空「{user.email}」的套餐和全部额度（时段 / 周 / 总额度），与「取消会员」等效。
              </span>
              <Button
                variant="outline" size="sm" disabled={busy !== ""}
                className="border-destructive/40 text-destructive hover:bg-destructive/10"
                onClick={savePlan}
              >
                {busy === "plan" ? "处理中…" : "确认退订"}
              </Button>
              <Button variant="ghost" size="sm" onClick={() => setConfirming(null)}>返回</Button>
            </div>
          )}

          <label className="mt-3 flex items-center gap-2 text-sm text-muted-foreground">
            <Checkbox
              checked={resetQuotas} disabled={plan === "none"}
              onChange={(e) => setResetQuotas(e.target.checked)}
            />
            按新套餐重置时段 / 周 / 总额度（不勾则保留他现有的额度；他本来就没有额度时，仍按套餐规格发一份，不会发出零额度会员）
          </label>
          {expiry === "" && plan !== "none" && (
            <p className="mt-2 text-xs text-muted-foreground">到期时间留空 = 永不过期。</p>
          )}

          <div className="mt-5 grid gap-3 sm:grid-cols-[minmax(0,10rem)_minmax(0,8rem)_auto] sm:items-end">
            <div>
              <Label htmlFor="extend">再加</Label>
              <Select id="extend" value={extendPlan} onChange={(e) => setExtendPlan(e.target.value)}>
                {PLANS.map((p) => <option key={p} value={p}>{p}</option>)}
              </Select>
            </div>
            <div>
              <Label htmlFor="days">时长（天）</Label>
              <Input
                id="days" type="number" min="1" value={extendDays}
                onChange={(e) => setExtendDays(e.target.value)}
              />
            </div>
            <Button variant="outline" size="sm" disabled={busy !== "" || extendValue <= 0} onClick={grantPlan}>
              {busy === "extend" ? "开通中…" : "叠加开通"}
            </Button>
          </div>
          <p className="mt-2 text-xs text-muted-foreground">
            叠加走的是和激活码同一条路：到期时间在现有基础上顺延，总额度累加，时段上限取两者较大值，
            不会把人降级。上面两个输入框会跟着刷新成叠加后的结果。
          </p>
        </section>
        </TabsContent>

        <TabsContent value="account" className="pt-4">
        <section>
          <h3 className="text-sm font-semibold">权限与账号</h3>
          <div className="mt-3 flex flex-wrap items-center gap-2">
            <Button
              variant="outline" size="sm" disabled={busy !== "" || isSelf}
              onClick={() =>
                act(
                  "role",
                  () => api.post<WriteResp>(`/api/admin/users/${user.id}/role`, {
                    role: user.role === "admin" ? "user" : "admin",
                  }),
                  user.role === "admin" ? "已取消管理员" : "已设为管理员",
                )
              }
            >
              {busy === "role" ? "处理中…" : user.role === "admin" ? "取消管理员" : "设为管理员"}
            </Button>

            {confirming === "cancel" ? (
              <>
                <span className="text-sm text-muted-foreground">退订会立即清零套餐和全部额度。</span>
                <Button
                  variant="outline" size="sm" disabled={busy !== ""}
                  className="border-destructive/40 text-destructive hover:bg-destructive/10"
                  onClick={cancelPlan}
                >
                  {busy === "cancel" ? "处理中…" : "确认取消会员"}
                </Button>
                <Button variant="ghost" size="sm" onClick={() => setConfirming(null)}>返回</Button>
              </>
            ) : (
              isActive(user) && (
                <Button
                  variant="outline" size="sm" disabled={busy !== ""}
                  className="border-destructive/40 text-destructive hover:bg-destructive/10"
                  onClick={() => { setMsg(null); setConfirming("cancel"); }}
                >
                  取消会员
                </Button>
              )
            )}

            {confirming === "delete" ? (
              <>
                <span className="text-sm text-muted-foreground">删除「{user.email}」后不可恢复。</span>
                <Button
                  variant="outline" size="sm" disabled={busy !== ""}
                  className="border-destructive/40 text-destructive hover:bg-destructive/10"
                  onClick={async () => {
                    setBusy("delete");
                    setMsg(null);
                    try {
                      await api.del<{ ok?: boolean }>(`/api/admin/users/${user.id}`);
                      await reload();
                      onClose();
                    } catch (e) {
                      setMsg({ text: e instanceof Error ? e.message : "删除失败", ok: false });
                      setBusy("");
                    }
                  }}
                >
                  {busy === "delete" ? "删除中…" : "确认删除"}
                </Button>
                <Button variant="ghost" size="sm" onClick={() => setConfirming(null)}>返回</Button>
              </>
            ) : (
              <Button
                variant="outline" size="sm" disabled={busy !== "" || isSelf}
                className="border-destructive/40 text-destructive hover:bg-destructive/10"
                onClick={() => { setMsg(null); setConfirming("delete"); }}
              >
                删除用户
              </Button>
            )}
          </div>
          {isSelf && (
            <p className="mt-2 text-xs text-muted-foreground">不能改自己的角色，也不能删自己的号。</p>
          )}
        </section>
        </TabsContent>
      </Tabs>
    </DialogContent>
  );
}
