import { useMemo, useState } from "react";
import { Search } from "lucide-react";
import { EmptyState } from "@/components/EmptyState";
import { PageHeader } from "@/components/PageHeader";
import { VendorMark, VENDOR_GROUPS } from "@/components/VendorMark";
import { SectionReveal } from "@/components/motion/section-reveal";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Stat } from "@/components/Stat";
import { num } from "@/lib/format";
import { cn } from "@/lib/utils";

/**
 * 模型图标 —— 后台认得出哪些厂商，这一页把它们全列出来。
 *
 * # 它不是装饰页，是一张对照表
 *
 * 多路由和线路那两屏会给每条线路自动配一个厂商图标，判据在服务端（`vendor_of`：先看
 * 模型 id，认不出来再看 base_url 的域名）。于是运维会遇到一个具体问题：**某条线路的
 * 图标是灰的，那是「我们没有这家的图」还是「判定没认出来」？** 这两种的处理方式完全
 * 不同 —— 前者只能等我加图，后者改一下模型 id 或地址就好了。
 *
 * 这一页回答的就是前半个问题：这里有的，就是后台认得出的全集。搜不到某一家，
 * 说明确实没有；搜得到但线路上没显示，那是判定没匹配上。
 *
 * # 为什么带搜索但不带分页
 *
 * 一百多个图标，滚一屏就到底了，分页只会多一次点击。搜索是给「这家到底有没有」用的 ——
 * 那才是来这一页的真正原因，所以搜索框在最上面，而且同时搜名字和标识符。
 */

export function VendorIcons() {
  const [q, setQ] = useState("");

  const total = useMemo(
    () => VENDOR_GROUPS.reduce((n, g) => n + g.items.length, 0),
    [],
  );

  const groups = useMemo(() => {
    const needle = q.trim().toLowerCase();
    if (!needle) return VENDOR_GROUPS;
    return VENDOR_GROUPS.map((g) => ({
      ...g,
      // 名字和标识符都搜：中文用户会搜「智谱」，而排查线路时手上拿到的是 `zhipu`。
      items: g.items.filter(
        (i) =>
          i.name.toLowerCase().includes(needle) || i.vendor.toLowerCase().includes(needle),
      ),
    })).filter((g) => g.items.length > 0);
  }, [q]);

  const hits = groups.reduce((n, g) => n + g.items.length, 0);

  return (
    <div className="space-y-6">
      <PageHeader
        title="模型图标"
        description="后台认得出的厂商全集。线路和多路由那两屏会自动配上这里的图——先看模型 id，认不出来再看地址的域名。"
      />

      <SectionReveal as="section" delay={70} className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <Stat label="图标" value={num(total)} hint="都是官方 logo" />
        {VENDOR_GROUPS.map((g) => (
          <Stat key={g.title} label={g.title} value={num(g.items.length)} />
        ))}
      </SectionReveal>

      <div className="relative max-w-sm">
        <Search className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
        <Input
          value={q}
          onChange={(e) => setQ(e.target.value)}
          placeholder="搜厂商名或标识符，如 智谱 / zhipu"
          className="pl-9"
          aria-label="搜索厂商图标"
        />
      </div>

      {q.trim() && !hits ? (
        <EmptyState
          title={`没有「${q.trim()}」`}
          hint="这一页就是全集——搜不到就是确实还没有这家的图标，线路上会显示成中性图标。"
        />
      ) : null}

      <SectionReveal as="section" delay={140} className="space-y-6">
        {groups.map((g) => (
          <div key={g.title}>
            <h2 className="mb-2 text-sm font-semibold">
              {g.title}
              <span className="ml-2 text-xs font-normal text-muted-foreground">
                {g.items.length}
              </span>
            </h2>
            <Card className="overflow-hidden">
              {/*
                自适应列数而不是写死几列：这一页会在很宽的屏上看（后台是桌面用的），
                写死 4 列的话右边会空掉一大片，而窄屏又会挤成两行。
              */}
              <div className="grid grid-cols-[repeat(auto-fill,minmax(11rem,1fr))]">
                {g.items.map((i, idx) => (
                  <div
                    key={i.vendor}
                    className={cn(
                      "flex items-center gap-3 border-border px-4 py-3",
                      // 只画上边线和左边线，靠 -mt/-ml 合并成一张网格，
                      // 免得相邻格子的边线叠成两像素粗。
                      idx >= 0 && "border-t border-l",
                    )}
                  >
                    <VendorMark vendor={i.vendor} />
                    <div className="min-w-0">
                      <p className="truncate text-[13px] font-medium">{i.name}</p>
                      <p className="truncate font-mono text-[11px] text-muted-foreground">
                        {i.vendor}
                      </p>
                    </div>
                  </div>
                ))}
              </div>
            </Card>
          </div>
        ))}
      </SectionReveal>

      <p className="text-xs leading-relaxed text-muted-foreground">
        图标取自开源图标集 <span className="font-mono">@lobehub/icons-static-svg</span>（MIT），
        抽出来内联在代码里，没有额外的运行时依赖。
        某条线路显示成中性图标时：这一页搜得到那家 = 判定没认出来（多半是模型 id
        或地址不带可识别的字样），搜不到 = 确实还没有这家的图。
      </p>
    </div>
  );
}
