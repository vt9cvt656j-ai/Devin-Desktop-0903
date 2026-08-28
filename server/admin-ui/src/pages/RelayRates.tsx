import { Fragment, useCallback, useEffect, useState } from "react";
import { Check, ChevronLeft, ChevronRight, Coins, Rabbit, Turtle } from "lucide-react";

import { ErrorState } from "@/components/ErrorState";
import { PageHeader } from "@/components/PageHeader";
import { Stat } from "@/components/Stat";
import { TableSkeleton } from "@/components/TableSkeleton";
import { SectionReveal } from "@/components/motion/section-reveal";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardHeader } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
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
 * 模型汇率 —— 每家中转充一块钱进去，能买到多少上游余额。
 *
 * # 这一屏为什么必须存在
 *
 * 多路由那边的「倍率」看起来是个纯数，其实带单位：**那家中转自己的余额单位**。
 * 而一块钱余额要花多少人民币，各家差几十倍。于是「0.05 倍」和「1.0 倍」谁便宜，
 * 光看倍率答不出来 —— 那是拿两种货币比大小。
 *
 * 而选路一直按倍率排序。也就是说在这一屏之前，它一直在按一个不可比的数
 * 挑「最便宜的门」，而且挑错了也不会有任何地方报错。
 *
 * 填上汇率之后，唯一可比的量才算得出来：
 *
 *     每一美元官方价实际花多少人民币 = 倍率 ÷ 汇率
 *
 * # 全有全无
 *
 * 一条线路下的候选出口**全部**知道汇率，选路才按人民币排；缺一个就整条退回按倍率排。
 * 把没填的当成 1.0 顶上去是最糟的选择：那会让一个纯粹「没填」的站凭空排到前面。
 */
type SiteUser = {
  route_label: string;
  cost_ratio: number;
  is_own: boolean;
  endpoint_label: string;
};

type Site = {
  host: string;
  users: SiteUser[];
  usd_per_cny: number | null;
  note: string;
  auto_rates: number[];
  cny_per_official_usd: number | null;
  best_ratio: number | null;
};

type Payload = {
  rows: Site[];
  sites: number;
  with_rate: number;
  all_known: boolean;
  /** 混合配比的统计窗口。页面上两处文案照它写，别再自己抄一个 30。 */
  mix_window_days: number;
};

const num = (v: number, d = 4) => Number(v.toFixed(d));

type Offer = {
  key: string;
  host: string;
  /** 走这一家、这一档价的线路。同一家同一个价被三条线路各挂一次不是三个选择。 */
  via: string[];
  endpoint_id: string;
  input_raw: number;
  output_raw: number;
  group_name: string;
  group_multiplier: number;
  source: "auto" | "manual";
  input_cny: number | null;
  output_cny: number | null;
  blended_cny: number | null;
  rank: number | null;
  probe_ms: number | null;
  probe_ok: boolean | null;
  fastest: boolean;
  slow: boolean;
};

type ModelRow = {
  model_id: string;
  open: boolean;
  mix_source: "usage" | "input_only";
  mix_in: number;
  mix_cached: number;
  mix_out: number;
  mix_calls: number;
  offers: Offer[];
  gap_pct: number | null;
};

type ModelPayload = {
  rows: ModelRow[];
  models: number;
  /** 有真实单价的模型数。 */
  priced: number;
  /** **两家以上**有价、真的比得了的模型数。和 priced 差得很远，必须分开说。 */
  comparable: number;
  open_models: number;
};

/** 名次说人话。用户要的就是「一低、二低」，不是「rank=1」。 */
const RANK_WORD = ["", "一低", "二低", "三低", "四低", "五低"];
const rankText = (n: number) => RANK_WORD[n] ?? `第 ${n} 低`;

const MODELS_PER_PAGE = 8;
/** 每个模型默认展开几家。挂十五个出口时，全列出来会把这一屏冲垮。 */
const OFFERS_SHOWN = 5;

export function RelayRates() {
  const [data, setData] = useState<Payload | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [draft, setDraft] = useState<Record<string, string>>({});
  const [busy, setBusy] = useState<string | null>(null);
  const [mp, setMp] = useState<ModelPayload | null>(null);
  const [q, setQ] = useState("");
  const [onlyOpen, setOnlyOpen] = useState(true);
  const [page, setPage] = useState(1);
  const [expanded, setExpanded] = useState<Record<string, boolean>>({});
  const [siteOpen, setSiteOpen] = useState<Record<string, boolean>>({});

  const load = useCallback(async () => {
    try {
      // 两个接口一起拉。逐模型比价依赖汇率，但它们是**两张表**：
      // 站级那张是填汇率的地方，模型那张是看填完之后谁真便宜的地方。
      const [sites, models] = await Promise.all([
        api.get<Payload>("/api/admin/relay-rates"),
        api.get<ModelPayload>("/api/admin/relay-model-prices"),
      ]);
      setData(sites);
      setMp(models);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "加载失败");
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function save(host: string) {
    setBusy(host);
    try {
      const raw = (draft[host] ?? "").trim();
      await api.post("/api/admin/relay-rates", {
        host,
        // 空 = 清掉这家站的汇率，回到按倍率排。不是 0 —— 0 会被当成一个真实汇率。
        usd_per_cny: raw === "" ? null : Number(raw),
      });
      setDraft((d) => {
        const n = { ...d };
        delete n[host];
        return n;
      });
      await load();
    } catch (e) {
      setError(e instanceof Error ? e.message : "保存失败");
    }
    setBusy(null);
  }

  const rows = data?.rows ?? [];

  /*
   * 每个站点有哪些模型、各是什么价。**从下面那份逐模型数据推出来，不另算一遍。**
   *
   * 上一版这里是站级自己算的一个数：`最低倍率 ÷ 汇率`。两个毛病：
   *   · 倍率是手填的近似值，而我们手上有从上游价目表真抓下来的逐模型单价；
   *   · 一家中转在不同模型上的便宜程度完全不同，压成一个数再宣布「最便宜」
   *     是把 478 个模型的结论替换成了 1 个模型的结论。
   *
   * `competitors > 1` 这道闸是关键：只有它一家有价的模型，说它「最低」没有意义。
   * 线上 478 个有价模型里只有 17 个在两家以上有价 —— 不区分的话，
   * 「在 400 个模型上最低」这种话会天天出现，而它什么都没说。
   */
  const perSite = new Map<
    string,
    {
      models: Array<{
        model_id: string;
        open: boolean;
        rank: number | null;
        competitors: number;
        input_cny: number | null;
        output_cny: number | null;
        blended_cny: number | null;
        input_raw: number;
        output_raw: number;
      }>;
      wins: number;
      comparable: number;
    }
  >();
  for (const m of mp?.rows ?? []) {
    for (const o of m.offers) {
      const e = perSite.get(o.host) ?? { models: [], wins: 0, comparable: 0 };
      e.models.push({
        model_id: m.model_id,
        open: m.open,
        rank: o.rank,
        competitors: m.offers.length,
        input_cny: o.input_cny,
        output_cny: o.output_cny,
        blended_cny: o.blended_cny,
        input_raw: o.input_raw,
        output_raw: o.output_raw,
      });
      if (m.offers.length > 1) {
        e.comparable += 1;
        if (o.rank === 1) e.wins += 1;
      }
      perSite.set(o.host, e);
    }
  }
  for (const e of perSite.values()) {
    // 有得比的排前面（那是真正有信息的），其余按名字。
    e.models.sort(
      (a, b) =>
        b.competitors - a.competitors ||
        (a.rank ?? 99) - (b.rank ?? 99) ||
        a.model_id.localeCompare(b.model_id),
    );
  }

  const needle = q.trim().toLowerCase();
  const filteredModels = (mp?.rows ?? []).filter(
    (m) => (!onlyOpen || m.open) && (!needle || m.model_id.toLowerCase().includes(needle)),
  );
  const modelPages = Math.max(1, Math.ceil(filteredModels.length / MODELS_PER_PAGE));
  const mPage = Math.min(page, modelPages);
  const shownModels = filteredModels.slice(
    (mPage - 1) * MODELS_PER_PAGE,
    mPage * MODELS_PER_PAGE,
  );

  return (
    <div className="space-y-4">
      <PageHeader
        title="模型汇率"
        description="每家中转充一块钱，能买到多少它自己的余额。填上它，才能跨中转比出谁真的便宜。"
      />

      {error && <ErrorState message={error} onRetry={() => void load()} />}

      {/*
        没读到就写「—」，不写 0。这三张卡渲染在骨架屏之前且不受 data 约束，
        原来加载中（和接口报错时）会显示「在用的中转站 0」「填了汇率 0 / 0」
        「选路现在按什么排：倍率（不可比）」—— 三句都是确定的陈述，
        而它们描述的是一个还没读到的世界。
        另外「选路按什么排」的真实判据是**逐线路**的（一条线路下的候选出口都填了汇率，
        那条就按人民币排），不是全局的 all_known，所以别再说「填全才会切过去」。
      */}
      <div className="grid gap-3 sm:grid-cols-3">
        <Stat label="在用的中转站" value={data ? data.sites : "—"} hint="从线路和出口的地址里认出来的" />
        <Stat
          label="填了汇率"
          value={data ? `${data.with_rate} / ${data.sites}` : "—"}
          hint={!data ? "读取中" : data.all_known ? "全部" : "没填的那些算不出真实成本"}
        />
        <Stat
          label="选路现在按什么排"
          value={!data ? "—" : data.all_known ? "真实人民币" : "看线路"}
          hint={
            !data
              ? "读取中"
              : data.all_known
                ? "跨中转可比"
                : "逐线路判：这条线下的候选出口都填了汇率，它就按人民币排"
          }
        />
      </div>

      {!data && !error && <TableSkeleton rows={5} />}

      {data && (
        <SectionReveal>
          <Card>
            <CardHeader className="pb-2">
              <p className="text-[13px] text-muted-foreground">
                <b>这张表是填汇率的地方</b>，右边「真实进价」点开就是这家站
                <b>每一个模型</b>的真实价 —— 一家中转在不同模型上的便宜程度完全不同，
                压成一个数再宣布谁最便宜是把几百个模型的结论替换成一个模型的结论。
                <br />
                多路由那边的<b>倍率</b>看着是个纯数，其实带单位——<b>那家中转自己的余额</b>。
                而一块钱余额要花多少人民币，各家差几十倍。所以「0.05 倍」和「1.0 倍」谁便宜，
                光看倍率<b>答不出来</b>，那是拿两种货币比大小。填上汇率，才算得出唯一可比的那个量：
                <br />
                <b className="font-mono">每一美元官方价花多少人民币 = 倍率 ÷ 汇率</b>
                <br />
                <b>全有全无</b>：一条线路下的候选出口全部填了汇率，选路才按人民币排；
                缺一个就整条线路退回按倍率排。把没填的当成 1 顶上去会让它凭空排到前面，
                而且不会有任何地方报错——所以宁可不换算。
              </p>
            </CardHeader>
            <div className="overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>中转站</TableHead>
                    <TableHead>谁在用它</TableHead>
                    <TableHead>充值汇率（¥1 买多少额度）</TableHead>
                    <TableHead>真实进价</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {rows.map((r) => {
                    const edited = draft[r.host] !== undefined;
                    const shown = edited
                      ? draft[r.host]
                      : r.usd_per_cny != null
                        ? String(num(r.usd_per_cny, 6))
                        : "";
                    const site = perSite.get(r.host);
                    return (
                      <Fragment key={r.host}>
                      <TableRow>
                        <TableCell>
                          <div className="flex items-center gap-2">
                            <Truncate className="font-mono text-[13px]" title={r.host}>
                              {r.host}
                            </Truncate>
                            {/*
                              「最便宜」那个全局徽章删了。它是从一个手填倍率除出来的
                              单一数字得出的结论，而一家中转在 478 个模型上的便宜程度
                              各不相同 —— 那句话在大多数模型上都是错的。
                              换成右边那一列「在几个可比模型上最低」，有分母、可核对。
                            */}
                          </div>
                          {r.auto_rates.length > 0 && (
                            <p className="mt-0.5 text-[11px] text-muted-foreground">
                              自动抓到 {r.auto_rates.length} 档套餐 · 比例{" "}
                              {num(r.auto_rates[0], 4)}
                              {r.auto_rates.length > 1 &&
                                ` – ${num(r.auto_rates[r.auto_rates.length - 1], 4)}`}
                            </p>
                          )}
                        </TableCell>

                        <TableCell className="text-[12px]">
                          {r.users.length === 0 ? (
                            <span className="text-muted-foreground">
                              已经没有线路在用（填了也不影响选路）
                            </span>
                          ) : (
                            <div className="space-y-0.5">
                              {r.users.map((u, i) => (
                                <div key={i} className="flex items-center gap-1.5">
                                  <span>{u.route_label}</span>
                                  <span className="text-muted-foreground">
                                    {u.is_own ? "自带地址" : u.endpoint_label || "出口"} ·{" "}
                                    {num(u.cost_ratio)}×
                                  </span>
                                </div>
                              ))}
                            </div>
                          )}
                        </TableCell>

                        <TableCell>
                          <div className="flex items-center gap-1.5">
                            <Input
                              className="h-8 w-28 text-xs"
                              placeholder="留空 = 不填"
                              value={shown}
                              onChange={(ev) =>
                                setDraft({ ...draft, [r.host]: ev.target.value })
                              }
                            />
                            <Button
                              size="sm"
                              variant={edited ? "default" : "outline"}
                              disabled={!edited || busy === r.host}
                              onClick={() => void save(r.host)}
                            >
                              {busy === r.host ? "存…" : <Check className="h-3.5 w-3.5" />}
                            </Button>
                          </div>
                          {r.note && (
                            <p className="mt-0.5 text-[11px] text-muted-foreground">{r.note}</p>
                          )}
                        </TableCell>

                        <TableCell className="text-[12px]">
                          {(() => {
                            const site = perSite.get(r.host);
                            if (!site || site.models.length === 0) {
                              return (
                                <span className="text-muted-foreground">
                                  没抓到逐模型价
                                  <span className="block text-[11px]">
                                    这家的价目表要么没公开，要么还没同步过
                                  </span>
                                </span>
                              );
                            }
                            return (
                              <div className="space-y-0.5">
                                <div>
                                  <b>{site.models.length}</b> 个模型有真实单价
                                </div>
                                <div className="text-[11px] text-muted-foreground">
                                  {site.comparable === 0 ? (
                                    "都只有这一家有价，没有比价对象"
                                  ) : (
                                    <>
                                      其中 {site.comparable} 个另有别家可比，
                                      <b className={cn(site.wins > 0 && "text-success")}>
                                        {site.wins} 个它最低
                                      </b>
                                    </>
                                  )}
                                </div>
                                <button
                                  className="text-[11px] text-muted-foreground underline-offset-2 hover:underline"
                                  onClick={() =>
                                    setSiteOpen((o) => ({ ...o, [r.host]: !o[r.host] }))
                                  }
                                >
                                  {siteOpen[r.host] ? "收起逐模型价" : "看逐模型价"}
                                </button>
                              </div>
                            );
                          })()}
                        </TableCell>
                      </TableRow>

                      {/*
                        展开：这家站**每一个**模型的真实价。
                        列了才叫「真实显示」—— 压成一个数再说谁最低，是把几百个模型的
                        结论替换成一个模型的结论。
                        有比价对象的排在前面，那些才是真正有信息的行。
                      */}
                      {siteOpen[r.host] && site && site.models.length > 0 && (
                        <TableRow className="hover:bg-transparent">
                          <TableCell colSpan={4} className="bg-muted/40 p-0">
                            <div className="max-h-96 overflow-y-auto">
                              <table className="w-full text-[12px]">
                                <thead className="sticky top-0 bg-muted">
                                  <tr className="text-[11px] text-muted-foreground">
                                    <th className="px-4 py-1.5 text-left font-normal">模型</th>
                                    <th className="w-28 px-2 py-1.5 text-right font-normal">
                                      输入 / 1M
                                    </th>
                                    <th className="w-28 px-2 py-1.5 text-right font-normal">
                                      输出 / 1M
                                    </th>
                                    <th className="w-40 px-4 py-1.5 text-left font-normal">
                                      和别家比
                                    </th>
                                  </tr>
                                </thead>
                                <tbody>
                                  {site.models.map((m) => (
                                    <tr
                                      key={m.model_id}
                                      className="border-t border-border/50"
                                    >
                                      <td className="px-4 py-1.5">
                                        <span className="font-mono">{m.model_id}</span>
                                        {m.open && (
                                          <span className="ml-1.5 text-[10px] text-success">
                                            已开放
                                          </span>
                                        )}
                                      </td>
                                      <td className="px-2 py-1.5 text-right tabular-nums">
                                        {m.input_cny != null
                                          ? `¥${num(m.input_cny, 3)}`
                                          : `原价 ${num(m.input_raw, 3)}`}
                                      </td>
                                      <td className="px-2 py-1.5 text-right tabular-nums">
                                        {m.output_cny != null
                                          ? `¥${num(m.output_cny, 3)}`
                                          : `原价 ${num(m.output_raw, 3)}`}
                                      </td>
                                      <td className="px-4 py-1.5">
                                        {m.competitors <= 1 ? (
                                          <span className="text-muted-foreground">
                                            只有这一家有
                                          </span>
                                        ) : m.rank === 1 ? (
                                          <span className="font-medium text-success">
                                            {m.competitors} 家里最低
                                          </span>
                                        ) : (
                                          <span className="text-muted-foreground">
                                            {m.competitors} 家里排第 {m.rank ?? "—"}
                                          </span>
                                        )}
                                      </td>
                                    </tr>
                                  ))}
                                </tbody>
                              </table>
                            </div>
                          </TableCell>
                        </TableRow>
                      )}
                      </Fragment>
                    );
                  })}
                </TableBody>
              </Table>
            </div>
          </Card>
        </SectionReveal>
      )}

      {mp && (
        <SectionReveal>
          <Card>
            <CardHeader className="pb-2">
              <div className="flex flex-wrap items-center justify-between gap-3">
                <h3 className="text-sm font-medium">
                  按模型比价
                  <span className="ml-2 text-xs font-normal text-muted-foreground">
                    {mp.models} 个模型有真实单价 · 其中 <b>{mp.comparable} 个两家以上比得了</b> ·
                    已开放 {mp.open_models} 个
                  </span>
                </h3>
                <div className="flex items-center gap-2">
                  <Input
                    className="h-8 w-44 text-xs"
                    placeholder="搜模型名"
                    value={q}
                    onChange={(ev) => {
                      setQ(ev.target.value);
                      setPage(1);
                    }}
                  />
                  <label className="flex cursor-pointer items-center gap-1.5 text-xs text-muted-foreground">
                    <input
                      type="checkbox"
                      checked={onlyOpen}
                      onChange={(ev) => {
                        setOnlyOpen(ev.target.checked);
                        setPage(1);
                      }}
                    />
                    只看已开放
                  </label>
                </div>
              </div>
              <p className="text-[13px] text-muted-foreground">
                站级那张表只能回答「这家整体便宜不便宜」，而钱是<b>按模型</b>花的——同一家中转，
                这个模型便宜、那个模型贵。这里用的是<b>从上游价目表真抓下来的逐模型单价</b>
                （{mp.models} 个模型），不是多路由那边手填的倍率。
                <br />
                名次按<b>混合价</b>排：<b className="font-mono">
                  输入价×输入占比 + 缓存价×缓存占比 + 输出价×输出占比
                </b>
                。占比不是我拍的，是这个模型<b>自己最近 {data?.mix_window_days ?? "…"} 天真实跑出来的</b>；
                还没有真实用量的模型会明说「按输入价排」，而不是编一个默认配比让排名看起来有依据。
                <br />
                <b>「有价」和「比得了」是两件事</b>：{mp.models} 个模型有真实单价，但只有{" "}
                {mp.comparable} 个在两家以上都有 —— 其余那些只有一家能提供，说它「最低」
                什么都没说，所以那些模型不排名次。
              </p>
            </CardHeader>

            <div className="space-y-2 px-5 pb-4">
              {shownModels.length === 0 && (
                <p className="py-6 text-center text-sm text-muted-foreground">
                  没有匹配的模型。{onlyOpen && "试试取消「只看已开放」。"}
                </p>
              )}
              {shownModels.map((m) => {
                const open = expanded[m.model_id];
                const list = open ? m.offers : m.offers.slice(0, OFFERS_SHOWN);
                // 只有一个选择时不排名次。给一个东西排「第一名」是没有信息的，
                // 而且会让人以为还有第二名可以比。
                const single = m.offers.length === 1;
                return (
                  <div key={m.model_id} className="overflow-hidden rounded-xl border border-border">
                    {/* 表头：模型名当标题，配比和差价当副标题。 */}
                    <div className="border-b border-border bg-muted/40 px-4 py-2.5">
                      <div className="flex flex-wrap items-center gap-2">
                        <span className="font-mono text-sm font-medium">{m.model_id}</span>
                        {m.open ? (
                          <Badge variant="success">已开放</Badge>
                        ) : (
                          <Badge variant="outline">没开放</Badge>
                        )}
                        {m.gap_pct != null && m.gap_pct >= 1 && (
                          <span className="ml-auto text-xs font-medium text-success">
                            用第一低比第二低省 {num(m.gap_pct, 0)}%
                          </span>
                        )}
                      </div>
                      <p className="mt-0.5 text-[11px] text-muted-foreground">
                        {single
                          ? "只有这一家能提供它 —— 没有比价空间"
                          : m.mix_source === "usage"
                            ? `混合价按真实配比：输入 ${Math.round(m.mix_in * 100)}% · 缓存 ${Math.round(
                                m.mix_cached * 100,
                              )}% · 输出 ${Math.round(m.mix_out * 100)}%（最近 ${data?.mix_window_days ?? "?"} 天 ${m.mix_calls} 次调用）`
                            : "这个模型还没跑过真实流量，混合价 = 输入价"}
                      </p>
                    </div>

                    {/* 表格：让数字上下对齐。上一版每行是一串挤在一起的文字，
                        三个长得差不多的价钱混在中间，谁也认不出哪个是排名依据。 */}
                    <div className="overflow-x-auto">
                      <table className="w-full text-[12px]">
                        <thead>
                          <tr className="border-b border-border text-[11px] text-muted-foreground">
                            {/*
                              名次这一列宽 5rem 而不是 4rem，徽章上还要 whitespace-nowrap：
                              中文在窄的 flex 项里会**逐字换行**，「一低」会被折成两行的
                              「一 / 低」。量出来才看得见，读代码看不出来。
                            */}
                            <th className="w-20 px-4 py-1.5 text-left font-normal">名次</th>
                            <th className="w-28 px-2 py-1.5 text-right font-normal">
                              混合价 / 1M
                            </th>
                            <th className="w-36 px-2 py-1.5 text-right font-normal">
                              输入 / 输出
                            </th>
                            <th className="px-2 py-1.5 text-left font-normal">中转站</th>
                            <th className="px-2 py-1.5 text-left font-normal">走这条的线路</th>
                            <th className="w-28 px-4 py-1.5 text-right font-normal">探测</th>
                          </tr>
                        </thead>
                        <tbody>
                          {list.map((o) => (
                            <tr
                              key={o.key}
                              className={cn(
                                "border-b border-border/60 last:border-b-0",
                                o.rank === 1 && !single && "bg-success/5",
                              )}
                            >
                              <td className="px-4 py-2">
                                {single ? (
                                  <span className="text-muted-foreground">—</span>
                                ) : o.rank != null ? (
                                  <Badge
                                    variant={o.rank === 1 ? "success" : "outline"}
                                    className="whitespace-nowrap"
                                  >
                                    {rankText(o.rank)}
                                  </Badge>
                                ) : (
                                  <span
                                    className="whitespace-nowrap text-muted-foreground"
                                    title="这家站没填充值汇率，算不出人民币价，所以没有名次"
                                  >
                                    没汇率
                                  </span>
                                )}
                              </td>

                              {/* 排名依据的那个数 —— 做成这一行里最大的字。 */}
                              <td className="px-2 py-2 text-right">
                                {o.blended_cny != null ? (
                                  <span
                                    className={cn(
                                      "text-[15px] font-semibold tabular-nums",
                                      o.rank === 1 && !single && "text-success",
                                    )}
                                  >
                                    ¥{num(o.blended_cny, 3)}
                                  </span>
                                ) : (
                                  <span className="text-muted-foreground">—</span>
                                )}
                              </td>

                              <td className="px-2 py-2 text-right tabular-nums text-muted-foreground">
                                {o.input_cny != null && o.output_cny != null ? (
                                  <>
                                    ¥{num(o.input_cny, 3)} / ¥{num(o.output_cny, 3)}
                                  </>
                                ) : (
                                  <span title="上游标的原价，还没换算成人民币">
                                    原价 {num(o.input_raw, 3)}/{num(o.output_raw, 3)}
                                  </span>
                                )}
                              </td>

                              <td className="max-w-[13rem] px-2 py-2">
                                <Truncate className="font-mono" title={o.host}>
                                  {o.host}
                                </Truncate>
                                {o.group_name && (
                                  <span className="block text-[11px] text-muted-foreground">
                                    {o.group_name}
                                    {o.group_multiplier !== 1 && ` ×${num(o.group_multiplier, 3)}`}
                                  </span>
                                )}
                              </td>

                              <td className="max-w-[18rem] px-2 py-2 text-muted-foreground">
                                <Truncate title={o.via.join("、")}>{o.via.join("、")}</Truncate>
                                {o.via.length > 1 && (
                                  <span
                                    className="block text-[11px]"
                                    title="这几条线路走的是同一家、同一档价，所以只算一个选择"
                                  >
                                    {o.via.length} 条线路共用
                                  </span>
                                )}
                              </td>

                              <td className="px-4 py-2 text-right">
                                <span className="inline-flex items-center gap-1.5">
                                  {o.probe_ms != null && (
                                    <span className="tabular-nums text-muted-foreground">
                                      {(o.probe_ms / 1000).toFixed(1)}s
                                    </span>
                                  )}
                                  {o.fastest && !single && (
                                    <Rabbit
                                      className="h-3.5 w-3.5 text-success"
                                      aria-label="最流畅"
                                    />
                                  )}
                                  {o.slow && (
                                    <Turtle
                                      className="h-3.5 w-3.5 text-warning"
                                      aria-label="慢"
                                    />
                                  )}
                                  {o.source === "manual" && (
                                    <Badge variant="outline" title="手录的价，不是抓来的">
                                      手录
                                    </Badge>
                                  )}
                                </span>
                              </td>
                            </tr>
                          ))}
                        </tbody>
                      </table>
                    </div>

                    {m.offers.length > OFFERS_SHOWN && (
                      <button
                        className="w-full border-t border-border px-3 py-1.5 text-[11px] text-muted-foreground hover:bg-accent/40"
                        onClick={() =>
                          setExpanded((e) => ({ ...e, [m.model_id]: !e[m.model_id] }))
                        }
                      >
                        {open ? "收起" : `还有 ${m.offers.length - OFFERS_SHOWN} 家，展开`}
                      </button>
                    )}
                  </div>
                );
              })}
            </div>

            {modelPages > 1 && (
              <div className="flex items-center justify-center gap-2 border-t border-border px-5 py-3 text-xs text-muted-foreground">
                <Button
                  size="sm"
                  variant="outline"
                  disabled={mPage <= 1}
                  onClick={() => setPage((p) => Math.max(1, p - 1))}
                >
                  <ChevronLeft className="h-3.5 w-3.5" /> 上一页
                </Button>
                <span className="tabular-nums">
                  第 {mPage} / {modelPages} 页 · 共 {filteredModels.length} 个模型
                </span>
                <Button
                  size="sm"
                  variant="outline"
                  disabled={mPage >= modelPages}
                  onClick={() => setPage((p) => Math.min(modelPages, p + 1))}
                >
                  下一页 <ChevronRight className="h-3.5 w-3.5" />
                </Button>
              </div>
            )}
          </Card>
        </SectionReveal>
      )}

      {data && (
        <p className="px-1 text-xs leading-relaxed text-muted-foreground">
          <Coins className="mr-1 inline h-3.5 w-3.5" />
          汇率怎么填：去中转的充值页看一档套餐，<b>到账额度 ÷ 付款人民币</b>就是这个数。
          比如「¥50 到账 $7」就填 <b className="font-mono">0.14</b>；
          「¥1 到账 500 额度」就填 <b className="font-mono">500</b>——
          单位跟着那家站的余额走，不必是美元，只要和它的倍率是同一套单位就对。
          有控制台令牌的站会自动抓套餐表，抓到了会显示在站点名下面，可以照着填。
        </p>
      )}
    </div>
  );
}
