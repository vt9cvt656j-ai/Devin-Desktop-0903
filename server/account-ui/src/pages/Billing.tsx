import { useEffect, useMemo, useState } from "react";
import { Check } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { api, type Catalog, type CatalogItem, type Me } from "@/lib/api";
import { creditsOf, currencySymbol, num, planIsActive, planLabel, price, usd, formatDate } from "@/lib/format";
import { DICTS, serverMessage, type Lang } from "@/lib/i18n";
import { cn } from "@/lib/utils";

type Props = {
  catalog: Catalog;
  me: Me | null;
  lang: Lang;
  onRedeemed: () => void;
};

/**
 * A product's name in the reader's language.
 *
 * Falls back through the base language ("zh-TW" → "zh") before English, so a Traditional
 * reader still gets a Simplified override if that is all the operator wrote — closer than
 * English, and the alternative was showing English to both.
 */
function localized(map: Record<string, string> | undefined, fallback: string, lang: Lang): string {
  if (!map) return fallback;
  return map[lang] ?? map[lang.split("-")[0]] ?? fallback;
}

const GROUPS = ["subscription", "credits", "redeem"] as const;
type Group = (typeof GROUPS)[number];

function Feature({ children }: { children: React.ReactNode }) {
  return (
    <li className="flex gap-2.5 text-[13.5px] leading-snug">
      <Check className="mt-0.5 size-[15px] flex-none text-success" />
      <span>{children}</span>
    </li>
  );
}

/*
 * How many columns a row of product cards should use.
 *
 * A fixed four-across grid orphans the last card whenever the count is not a multiple of
 * four: five plans rendered as four across and one stranded underneath. The count is not
 * fixed either — it is however many active prices Stripe has — so the layout has to be
 * chosen from it rather than written once.
 *
 * The rule is to avoid a last row holding a single card. Four-across leaves an orphan
 * whenever `n % 4 == 1`, so those counts drop to three (5 → 3+2, 9 → 3+3+3). Six is
 * three as well: 3+3 reads as a deliberate pair of rows where 4+2 reads as a spill.
 */
function columnsFor(n: number): 1 | 2 | 3 | 4 {
  if (n <= 4) return (Math.max(n, 1) as 1 | 2 | 3 | 4);
  if (n % 4 === 1 || n === 6) return 3;
  return 4;
}

/*
 * Widths per column count, as whole class strings.
 *
 * Tailwind scans source text for class names, so these cannot be built by interpolation —
 * `w-[calc(${x}%)]` produces no CSS at all. Each width subtracts its share of the 1rem
 * gap: n columns carry (n-1) gaps between them.
 *
 * Flex rather than grid, because `justify-center` then centres a short final row. CSS
 * grid would leave those two cards hard against the left edge with a column-shaped hole
 * beside them, which is the same ugliness in a different place.
 */
const CARD_WIDTH: Record<number, string> = {
  1: "sm:w-[380px]",
  2: "sm:w-[calc(50%-0.5rem)]",
  3: "sm:w-[calc(50%-0.5rem)] lg:w-[calc(33.333%-0.667rem)]",
  4: "sm:w-[calc(50%-0.5rem)] xl:w-[calc(25%-0.75rem)]",
};

/** A tier or pack. The CTA is passed in because each group buys differently. */
function ProductCard({
  name,
  badge,
  badgeVariant,
  highlighted,
  priceMain,
  priceUnit,
  features,
  action,
  className,
}: {
  name: string;
  badge?: string;
  badgeVariant?: "default" | "success";
  highlighted?: boolean;
  priceMain: React.ReactNode;
  priceUnit?: string;
  features: React.ReactNode[];
  action: React.ReactNode;
  /** The row's per-column width. The card is the flex item, so it carries its own. */
  className?: string;
}) {
  return (
    <Card
      className={cn(
        // Full width until the first breakpoint, and never wider than a comfortable
        // reading column — otherwise two plans on a wide screen become two billboards.
        "w-full max-w-[380px] p-6 transition-colors",
        highlighted && "border-primary/45",
        className,
      )}
    >
      {/*
        * Two lines' worth of height whether the name needs one or two.
        *
        * Names come from Stripe and vary in length, so "Credit Package A" fitted on one
        * line while "Credit Package C" wrapped onto two — which pushed that card's price
        * a line lower than its neighbours'. Nothing lined up across the row. Reserving
        * the space means the prices sit on one baseline regardless.
        */}
      <div className="flex min-h-[3.25rem] items-start justify-between gap-2.5">
        <span className="text-[17px] font-semibold leading-snug tracking-tight">{name}</span>
        {badge ? (
          // Never broken across lines: "Best rate" wrapping to "Best / rate" both looked
          // wrong and stole width from the name, causing the wrap described above.
          <Badge
            variant={badgeVariant === "success" ? "success" : "default"}
            className="whitespace-nowrap"
          >
            {badge}
          </Badge>
        ) : null}
      </div>

      <div className="mb-5 mt-1 flex items-baseline gap-1.5">
        <span className="text-[34px] font-semibold tracking-tighter tabular-nums">{priceMain}</span>
        {priceUnit ? <span className="text-sm font-medium text-muted-foreground">{priceUnit}</span> : null}
      </div>

      <ul className="mb-6 flex flex-col gap-2.5">{features}</ul>
      <div className="mt-auto">{action}</div>
    </Card>
  );
}

/**
 * A one-off offer, laid out as a full-width row rather than a card in the grid.
 *
 * The day pass and the custom top-up are not comparable to the things beside them — one
 * is a 24-hour pass among monthly tiers, the other is an input box among fixed packs.
 * Dropping them into the same row of equal cards forced a shape onto content that does
 * not have it, and left the row looking ragged no matter how the columns were counted.
 * Given their own row they stop competing with the set above and read as what they are:
 * the alternative to it.
 */
function OfferStrip({
  name,
  detail,
  amount,
  unit,
  action,
}: {
  name: string;
  detail: React.ReactNode;
  amount: React.ReactNode;
  unit?: string;
  action: React.ReactNode;
}) {
  return (
    <Card className="flex flex-col gap-5 p-6 sm:flex-row sm:items-center sm:gap-8">
      <div className="min-w-0 flex-1">
        <div className="text-[15px] font-semibold tracking-tight">{name}</div>
        <div className="mt-1.5 text-[13.5px] leading-relaxed text-muted-foreground">{detail}</div>
      </div>
      <div className="flex items-baseline gap-1.5 sm:justify-end">
        <span className="text-[26px] font-semibold tracking-tight tabular-nums">{amount}</span>
        {unit ? <span className="text-sm font-medium text-muted-foreground">{unit}</span> : null}
      </div>
      <div className="sm:w-40">{action}</div>
    </Card>
  );
}

function Row({ k, v }: { k: string; v: React.ReactNode }) {
  return (
    <div className="flex items-baseline justify-between gap-4 border-b border-border/60 py-2.5 last:border-b-0">
      <span className="text-[13px] text-muted-foreground">{k}</span>
      <span className="text-[13px] font-medium tabular-nums">{v}</span>
    </div>
  );
}

/** Quiet heading above each set, so the two halves of a tab read as deliberate groups. */
function SetLabel({ children }: { children: React.ReactNode }) {
  return <h2 className="mb-3 mt-8 text-sm font-semibold">{children}</h2>;
}

export function Billing({ catalog, me, lang, onRedeemed }: Props) {
  const t = DICTS[lang];
  const [group, setGroup] = useState<Group>(() => {
    const h = location.hash.replace("#", "") as Group;
    return GROUPS.includes(h) ? h : "subscription";
  });
  const [busy, setBusy] = useState<string | null>(null);
  const [message, setMessage] = useState<{ text: string; kind: "ok" | "err" } | null>(null);
  const [quantity, setQuantity] = useState(50);
  const [code, setCode] = useState("");

  useEffect(() => {
    const params = new URLSearchParams(location.search);
    if (params.get("paid")) {
      setMessage({ text: t.paidOk, kind: "ok" });
      history.replaceState(null, "", "/billing");
    } else if (params.get("canceled")) {
      setMessage({ text: t.canceled, kind: "err" });
      history.replaceState(null, "", "/billing");
    }
  }, [t]);

  const { plans, passes, packs, custom } = useMemo(() => {
    const items = catalog.items ?? [];
    return {
      plans: items.filter((i) => i.kind === "plan" && i.recurring),
      passes: items.filter((i) => i.kind === "plan" && !i.recurring),
      packs: items.filter((i) => i.kind === "credits" && !i.unit_credits_cents),
      custom: items.find((i) => i.unit_credits_cents) ?? null,
    };
  }, [catalog]);

  const currentPlan = me && planIsActive(me.plan, me.plan_expires_at) ? me.plan : null;

  const bestPackKey = useMemo(() => {
    // Against the price on show, for the same reason the rate line is: comparing packs
    // by a stored column while quoting a Stripe price could crown a "best rate" that is
    // not the best at the prices anybody actually sees.
    let best: { rate: number; key: string | null } = { rate: 0, key: null };
    for (const p of packs) {
      const minor = p.display_minor ?? 0;
      const rate = minor > 0 ? (p.credits_cents ?? 0) / minor : 0;
      if (rate > best.rate) best = { rate, key: p.lookup_key };
    }
    return best.key;
  }, [packs]);

  async function buy(item: CatalogItem, qty?: number) {
    setBusy(item.lookup_key);
    setMessage(null);
    try {
      const res = await api.checkout(item.lookup_key, qty);
      if (!res.url) throw new Error(t.checkoutNoUrl);
      location.assign(res.url);
    } catch (e) {
      setMessage({ text: serverMessage(e, lang), kind: "err" });
      setBusy(null);
    }
  }

  async function redeem() {
    if (!code.trim()) return;
    setBusy("redeem");
    setMessage(null);
    try {
      await api.redeem(code.trim());
      setCode("");
      setMessage({ text: t.redeemOk, kind: "ok" });
      onRedeemed();
    } catch (e) {
      setMessage({ text: serverMessage(e, lang), kind: "err" });
    }
    setBusy(null);
  }

  const disabled = !catalog.enabled;

  return (
    <div className="mx-auto max-w-[1180px]">
      <header className="mb-5 text-center">
        <h1 className="text-[25px] font-semibold tracking-tight">{t.billing}</h1>
        <p className="mx-auto mt-1.5 max-w-[60ch] text-[13.5px] leading-relaxed text-muted-foreground">
          {t.billingLede}
        </p>
      </header>

      <Tabs
        value={group}
        onValueChange={(v) => {
          setGroup(v as Group);
          history.replaceState(null, "", `/billing#${v}`);
          setMessage(null);
        }}
        className="items-center"
      >
        <TabsList className="mx-auto">
          <TabsTrigger value="subscription">{t.tabSubscription}</TabsTrigger>
          <TabsTrigger value="credits">{t.tabCredits}</TabsTrigger>
          <TabsTrigger value="redeem">{t.tabRedeem}</TabsTrigger>
        </TabsList>
      </Tabs>

      {me ? (
        <p className="mt-5 rounded-xl border border-border bg-muted px-4 py-3 text-[13px] text-muted-foreground">
          {currentPlan ? (
            <>
              {t.youAreOn} <strong className="font-semibold text-foreground">{planLabel(me.plan, lang)}</strong>
              {me.plan_expires_at ? (
                <>
                  {" · "}
                  {t.until}{" "}
                  <strong className="font-semibold text-foreground">
                    {formatDate(me.plan_expires_at, lang)}
                  </strong>
                </>
              ) : null}{" "}
              {t.extends}
            </>
          ) : (
            <>
              {t.freePlanBalance}{" "}
              <strong className="font-semibold text-foreground">{usd(me.credits_cents)}</strong>.
            </>
          )}
        </p>
      ) : null}

      {disabled && group !== "redeem" ? (
        <p className="mt-3 rounded-xl border border-destructive/30 bg-destructive/5 px-4 py-3 text-[13px] text-destructive">
          {t.notEnabled}
        </p>
      ) : null}

      {group === "subscription" ? (
        <>
          <SetLabel>{t.monthlyPlans}</SetLabel>
          <div className="flex flex-wrap items-stretch justify-center gap-4">
          {plans.map((p) => {
            const mine = !!currentPlan && p.plan === currentPlan && p.recurring;
            const width = CARD_WIDTH[columnsFor(plans.length)];
            return (
              <ProductCard
                key={p.lookup_key}
                className={width}
                name={localized(p.labels, p.label, lang)}
                badge={mine ? t.current : undefined}
                highlighted={mine}
                priceMain={price(p)}
                // How long it lasts, not how it is billed. `recurring` says whether
                // Stripe charges again; a one-off payment can still buy 30 days, and
                // reading it as "day pass" labelled such a plan /天 with "24 hours"
                // beside a 30-day grant.
                priceUnit={t.perMonth}
                features={
                  p.duration_days !== 1
                    ? [
                        <Feature key="inc">
                          <strong className="font-semibold tabular-nums">{usd(p.included_cents)}</strong>{" "}
                          <span className="text-muted-foreground">{t.includedEachMonth}</span>
                        </Feature>,
                        // Only when there is a cap. A plan with none rendered
                        // "$0.00 per 5½-hour window", which reads as a plan that gives
                        // you nothing — the opposite of what an absent cap means.
                        ...(p.window_cap_cents
                          ? [
                              <Feature key="win">
                                <strong className="font-semibold tabular-nums">
                                  {usd(p.window_cap_cents)}
                                </strong>{" "}
                                <span className="text-muted-foreground">{t.per55}</span>
                              </Feature>,
                            ]
                          : []),
                        <Feature key="wk">
                          <span className="text-muted-foreground">
                            {p.weekly_cap_cents ? `${usd(p.weekly_cap_cents)} ${t.weeklyCapSuffix}` : t.noWeeklyCap}
                          </span>
                        </Feature>,
                        <Feature key="models">
                          <span className="text-muted-foreground">{t.allModels}</span>
                        </Feature>,
                      ]
                    : [
                        <Feature key="day">
                          <strong className="font-semibold tabular-nums">{usd(p.included_cents)}</strong>{" "}
                          <span className="text-muted-foreground">{t.fullDayQuota}</span>
                        </Feature>,
                        // Only where it is true — this used to be printed on every
                        // non-subscription product regardless.
                        ...(p.once_per_account
                          ? [
                              <Feature key="once">
                                <span className="text-muted-foreground">{t.onePerAccount}</span>
                              </Feature>,
                            ]
                          : []),
                      ]
                }
                action={
                  <Button
                    className="w-full"
                    variant={p.already_purchased ? "secondary" : "default"}
                    disabled={disabled || p.already_purchased || busy === p.lookup_key}
                    onClick={() => buy(p)}
                  >
                    {p.already_purchased ? t.alreadyUsed : busy === p.lookup_key ? t.opening : t.subscribe}
                  </Button>
                }
              />
            );
          })}
          </div>

          {passes.length ? (
            <>
              <SetLabel>{t.dayPassSet}</SetLabel>
              <div className="flex flex-col gap-3">
                {passes.map((p) => (
                  <OfferStrip
                    key={p.lookup_key}
                    name={localized(p.labels, p.label, lang)}
                    detail={
                      <>
                        <strong className="font-semibold text-foreground tabular-nums">
                          {usd(p.included_cents)}
                        </strong>{" "}
                        {t.fullDayQuota}
                        {p.once_per_account ? ` · ${t.onePerAccount}` : ""}
                      </>
                    }
                    amount={price(p)}
                    unit={t.perDay}
                    action={
                      <Button
                        className="w-full"
                        variant={p.already_purchased ? "secondary" : "default"}
                        disabled={disabled || p.already_purchased || busy === p.lookup_key}
                        onClick={() => buy(p)}
                      >
                        {p.already_purchased
                          ? t.alreadyUsed
                          : busy === p.lookup_key
                            ? t.opening
                            : t.subscribe}
                      </Button>
                    }
                  />
                ))}
              </div>
            </>
          ) : null}
        </>
      ) : null}

      {group === "credits" ? (
        <>
          <SetLabel>{t.creditPacks}</SetLabel>
          <div className="flex flex-wrap items-stretch justify-center gap-4">
          {packs.map((p) => {
            const credits = creditsOf(p.credits_cents);
            const width = CARD_WIDTH[columnsFor(packs.length)];
            // Rate against the price actually shown, in that price's own currency. It
            // used to divide by the stored yuan column, so a card read "$5.99" above
            // "3.33 per ¥1" — two different prices for one product.
            const major = (p.display_minor ?? 0) / 100;
            const rate = major > 0 ? (credits / major).toFixed(2) : null;
            return (
              <ProductCard
                key={p.lookup_key}
                className={width}
                name={localized(p.labels, p.label, lang)}
                badge={p.lookup_key === bestPackKey ? t.bestRate : undefined}
                badgeVariant="success"
                priceMain={price(p)}
                features={[
                  <Feature key="c">
                    <strong className="font-semibold tabular-nums">{num(credits)}</strong>{" "}
                    <span className="text-muted-foreground">{t.credits}</span>
                  </Feature>,
                  ...(rate
                    ? [
                        <Feature key="r">
                          <span className="tabular-nums text-muted-foreground">
                            {`${rate} ${t.per} ${currencySymbol(p)}1`}
                          </span>
                        </Feature>,
                      ]
                    : []),
                  <Feature key="e">
                    <span className="text-muted-foreground">{t.neverExpires}</span>
                  </Feature>,
                ]}
                action={
                  <Button
                    className="w-full"
                    disabled={disabled || busy === p.lookup_key}
                    onClick={() => buy(p)}
                  >
                    {busy === p.lookup_key ? t.opening : t.buy}
                  </Button>
                }
              />
            );
          })}

          </div>

          {custom ? (
            <>
              <SetLabel>{t.customAmount}</SetLabel>
              <OfferStrip
                name={localized(custom.labels, custom.label, lang)}
                detail={
                  <>
                    <strong className="font-semibold text-foreground tabular-nums">
                      {num(creditsOf((custom.unit_credits_cents ?? 0) * quantity))}
                    </strong>{" "}
                    {t.credits} · {t.neverExpires}
                  </>
                }
                amount={
                  <span className="flex items-baseline gap-1.5">
                    <span className="text-lg text-muted-foreground">{currencySymbol(custom)}</span>
                    <Input
                      type="number"
                      min={1}
                      max={100000}
                      value={quantity}
                      aria-label={t.customAmount}
                      onChange={(e) =>
                        setQuantity(Math.max(1, Math.min(100000, Number.parseInt(e.target.value, 10) || 1)))
                      }
                      className="h-11 w-28 text-lg font-semibold tabular-nums"
                    />
                  </span>
                }
                action={
                  <Button
                    className="w-full"
                    disabled={disabled || busy === custom.lookup_key}
                    onClick={() => buy(custom, quantity)}
                  >
                    {busy === custom.lookup_key ? t.opening : t.topUp}
                  </Button>
                }
              />
            </>
          ) : null}
        </>
      ) : null}

      {group === "redeem" ? (
        /*
         * Deliberately small.
         *
         * Redeeming is one short action — paste a code, press a button — and it was
         * rendered as a 600px card holding a 64px field above a 56px button, two slabs
         * of furniture for two controls with nothing else on the tab to balance them.
         * Sizing a control says how big the job is, and this job is small: the field and
         * its button share one line, at the same height as every other control in the
         * console, and the note that was floating outside comes inside where it belongs.
         */
        /*
         * A page, not a lone box.
         *
         * The form was sized up twice and still looked wrong, because the problem was
         * never its size — the other two tabs are dense with plans and packs while this
         * one held a single card in an otherwise empty page. Nothing to compare it to
         * makes any size look arbitrary. So the tab now carries what it always should
         * have: the form, what the account holds right now for a code to add to, and
         * what each kind of code actually does.
         */
        <div className="mt-8 grid grid-cols-1 gap-4 lg:grid-cols-5">
          <Card className="p-8 lg:col-span-3">
            <h2 className="text-lg font-semibold tracking-tight">{t.redeemTitle}</h2>
            <Label htmlFor="code" className="mb-2.5 mt-6 block text-sm text-muted-foreground">
              {t.activationCode}
            </Label>
            <div className="flex flex-col gap-3 sm:flex-row">
              <Input
                id="code"
                value={code}
                autoComplete="off"
                spellCheck={false}
                placeholder="XXXX-XXXX-XXXX"
                onChange={(e) => setCode(e.target.value.toUpperCase())}
                onKeyDown={(e) => {
                  if (e.key === "Enter") void redeem();
                }}
                // Uppercase and lightly tracked so a pasted code stays readable.
                className="h-13 flex-1 text-base font-medium tracking-[0.08em] uppercase"
              />
              <Button
                size="lg"
                className="h-13 shrink-0 text-base sm:w-32"
                disabled={busy === "redeem" || !code.trim()}
                onClick={() => void redeem()}
              >
                {t.redeem}
              </Button>
            </div>
            <p className="mt-4 text-[13px] leading-relaxed text-muted-foreground">{t.redeemNote}</p>
          </Card>

          {/* What a code would be adding to. Every figure is the account's own. */}
          <Card className="bg-muted p-6 lg:col-span-2">
            <h2 className="mb-1 text-sm font-semibold">{t.accountNow}</h2>
            <div className="mt-2">
              <Row k={t.currentPlan} v={me && currentPlan ? planLabel(me.plan, lang) : t.noPlan} />
              {me?.plan_expires_at ? (
                <Row k={t.expires} v={formatDate(me.plan_expires_at, lang)} />
              ) : null}
              <Row k={t.creditBalance} v={usd(me?.credits_cents ?? 0)} />
              <Row
                k={t.dailyFree}
                v={`${Math.round((me?.free_points ?? 0) * 100) / 100} / ${me?.free_points_daily ?? 0}`}
              />
            </div>
          </Card>

          {/* The two kinds of code that exist, described by what redeeming one does. */}
          <Card className="p-6 lg:col-span-5">
            <div className="grid grid-cols-1 gap-6 sm:grid-cols-2">
              <div>
                <h3 className="text-sm font-semibold">{t.planCodeTitle}</h3>
                <p className="mt-1.5 text-[13px] leading-relaxed text-muted-foreground">
                  {t.planCodeBody}
                </p>
              </div>
              <div>
                <h3 className="text-sm font-semibold">{t.creditCodeTitle}</h3>
                <p className="mt-1.5 text-[13px] leading-relaxed text-muted-foreground">
                  {t.creditCodeBody}
                </p>
              </div>
            </div>
          </Card>
        </div>
      ) : null}

      {message ? (
        <p
          className={cn(
            "mt-4 text-center text-[13px]",
            message.kind === "ok" ? "text-success" : "text-destructive",
          )}
        >
          {message.text}
        </p>
      ) : null}

      {group !== "redeem" ? (
        <p className="mt-7 text-center text-xs text-muted-foreground">{t.secure}</p>
      ) : null}
    </div>
  );
}
