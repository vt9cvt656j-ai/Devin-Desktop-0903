/**
 * 订单金额的唯一口径。
 *
 * # 为什么单独一个文件
 *
 * 这套判断原来在收款页有一份、总览页有一份。收款页那份修过一半（会看 charged_cents 了），
 * 总览页那份没跟上 —— 而总览页的注释里还写着「Billing.tsx 已经改过」。两份抄在一起的东西
 * 只要有一处先改，另一处就会带着一句自信的注释继续错下去。
 *
 * # 什么是真钱
 *
 * 只有 `charged_cents` + `charged_currency` 是真钱：Stripe 扣款成功后由 webhook 写回来的事实。
 *
 * `amount_cents` **不是**。它是 `prices.amount_cents × 份数`，也就是目录里的**人民币**标价，
 * 对美元买家也一样绑人民币价（server/src/stripe.rs 的 INSERT）。拿它当营收累加，等于把
 * 没成交的订单和没进账的手工发放也算成钱，还会把人民币当美元报出去。
 *
 * `resolved_currency` 也不是。它是下单时**打算**按哪个币收（按 IP/语言/时区猜的），
 * 而 charge_ccy=usd 时根本不给 Stripe 传 currency，Stripe 会按该价格的 base currency 结算。
 */

/** 一笔订单里和钱有关的那几个字段。两屏各有各的 Order 类型，所以这里用结构类型。 */
export type Charged = {
  charged_cents?: number | null;
  charged_currency?: string | null;
  refunded_at?: string | null;
};

/**
 * 把最小单位金额写成人看的形式。**币种必须显式传，没有默认值** ——
 * 一个写死的 `$` 就是这两屏原来最大的谎：库里 42590 是人民币，页面上写着 $425.90。
 */
export function formatMoney(minor: number, ccy: string): string {
  const n = (minor / 100).toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
  const c = (ccy || "").toLowerCase();
  if (c === "cny") return `¥${n}`;
  if (c === "usd") return `$${n}`;
  // 没见过的币种就把代码写出来。难看好过挂一个可能是错的符号。
  return c ? `${n} ${c.toUpperCase()}` : n;
}

/** 这笔订单真正收到的钱。拿不到就是 null —— **不退回标价**，标价不是钱。 */
export function realCharge(o: Charged) {
  return typeof o.charged_cents === "number" && o.charged_currency
    ? { minor: o.charged_cents, ccy: o.charged_currency.toLowerCase() }
    : null;
}

/** 按币种分桶累加。跨币种相加要先定用哪天的汇率，在定下来之前加出来的数是假的。 */
export function sumByCurrency(rows: Charged[]): Record<string, number> {
  return rows.reduce<Record<string, number>>((m, o) => {
    const real = realCharge(o);
    if (real) m[real.ccy] = (m[real.ccy] || 0) + real.minor;
    return m;
  }, {});
}

/** 分桶结果写成一行，大额在前。一个桶都没有就写「—」，不写 0（没有进账 ≠ 收了 0 块）。 */
export function formatTotals(byCcy: Record<string, number>): string {
  const parts = Object.entries(byCcy).sort((a, b) => b[1] - a[1]);
  return parts.length ? parts.map(([c, v]) => formatMoney(v, c)).join(" + ") : "—";
}
