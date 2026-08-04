import { useEffect, useMemo, useState } from "react";
import { Check } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { api, type Catalog, type CatalogItem, type Me } from "@/lib/api";
import { creditsOf, planIsActive, planLabel, price, priceAlt, usd, formatDate } from "@/lib/format";
import { DICTS, serverMessage, type Currency, type Lang } from "@/lib/i18n";
import { cn } from "@/lib/utils";

type Props = {
  catalog: Catalog;
  me: Me | null;
  lang: Lang;
  currency: Currency;
  onRedeemed: () => void;
};

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

/** A tier or pack. The CTA is passed in because each group buys differently. */
function ProductCard({
  name,
  badge,
  badgeVariant,
  highlighted,
  priceMain,
  priceUnit,
  priceAltText,
  features,
  action,
}: {
  name: string;
  badge?: string;
  badgeVariant?: "default" | "success";
  highlighted?: boolean;
  priceMain: React.ReactNode;
  priceUnit?: string;
  priceAltText?: string;
  features: React.ReactNode[];
  action: React.ReactNode;
}) {
  return (
    <Card className={cn("p-6 transition-colors", highlighted && "border-primary/45")}>
      <div className="flex min-h-6 items-start justify-between gap-2.5">
        <span className="text-[17px] font-semibold tracking-tight">{name}</span>
        {badge ? (
          <Badge variant={badgeVariant === "success" ? "success" : "default"}>{badge}</Badge>
        ) : null}
      </div>

      <div className="mt-4 flex items-baseline gap-1.5">
        <span className="text-[34px] font-semibold tracking-tighter tabular-nums">{priceMain}</span>
        {priceUnit ? <span className="text-sm font-medium text-muted-foreground">{priceUnit}</span> : null}
      </div>
      <p className="mb-5 mt-0.5 min-h-4 text-xs text-muted-foreground tabular-nums">{priceAltText}</p>

      <ul className="mb-6 flex flex-col gap-2.5">{features}</ul>
      <div className="mt-auto">{action}</div>
    </Card>
  );
}

export function Billing({ catalog, me, lang, currency, onRedeemed }: Props) {
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
    let best: { rate: number; key: string | null } = { rate: 0, key: null };
    for (const p of packs) {
      const rate = p.amount_cents > 0 ? (p.credits_cents ?? 0) / p.amount_cents : 0;
      if (rate > best.rate) best = { rate, key: p.lookup_key };
    }
    return best.key;
  }, [packs]);

  async function buy(item: CatalogItem, qty?: number) {
    setBusy(item.lookup_key);
    setMessage(null);
    try {
      const res = await api.checkout(item.lookup_key, qty);
      if (!res.url) throw new Error("Stripe did not return a checkout link.");
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
              {t.youAreOn} <strong className="font-semibold text-foreground">{planLabel(me.plan)}</strong>
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
        <div className="mt-6 grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-4">
          {[...passes, ...plans].map((p) => {
            const mine = !!currentPlan && p.plan === currentPlan && p.recurring;
            return (
              <ProductCard
                key={p.lookup_key}
                name={p.label}
                badge={mine ? t.current : undefined}
                highlighted={mine}
                priceMain={price(p, currency)}
                priceUnit={p.recurring ? t.perMonth : t.perDay}
                priceAltText={priceAlt(p, currency)}
                features={
                  p.recurring
                    ? [
                        <Feature key="inc">
                          <strong className="font-semibold tabular-nums">{usd(p.included_cents)}</strong>{" "}
                          <span className="text-muted-foreground">{t.includedEachMonth}</span>
                        </Feature>,
                        <Feature key="win">
                          <strong className="font-semibold tabular-nums">{usd(p.window_cap_cents)}</strong>{" "}
                          <span className="text-muted-foreground">{t.per55}</span>
                        </Feature>,
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
                        <Feature key="once">
                          <span className="text-muted-foreground">{t.onePerAccount}</span>
                        </Feature>,
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
      ) : null}

      {group === "credits" ? (
        <div className="mt-6 grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-4">
          {packs.map((p) => {
            const credits = creditsOf(p.credits_cents);
            const rate = p.amount_cents > 0 ? (credits / (p.amount_cents / 100)).toFixed(2) : "0";
            return (
              <ProductCard
                key={p.lookup_key}
                name={p.label}
                badge={p.lookup_key === bestPackKey ? t.bestRate : undefined}
                badgeVariant="success"
                priceMain={price(p, currency)}
                priceAltText={priceAlt(p, currency)}
                features={[
                  <Feature key="c">
                    <strong className="font-semibold tabular-nums">{credits.toLocaleString()}</strong>{" "}
                    <span className="text-muted-foreground">{t.credits}</span>
                  </Feature>,
                  <Feature key="r">
                    <span className="tabular-nums text-muted-foreground">{`${rate} ${t.perYuan}`}</span>
                  </Feature>,
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

          {custom ? (
            <ProductCard
              name={t.customAmount}
              priceMain={
                <span className="flex items-baseline gap-1">
                  <span className="text-lg text-muted-foreground">¥</span>
                  <Input
                    type="number"
                    min={1}
                    max={100000}
                    value={quantity}
                    aria-label={t.customAmount}
                    onChange={(e) =>
                      setQuantity(Math.max(1, Math.min(100000, Number.parseInt(e.target.value, 10) || 1)))
                    }
                    className="h-11 w-full text-lg font-semibold tabular-nums"
                  />
                </span>
              }
              priceAltText={
                currency === "usd"
                  ? `¥${quantity}`
                  : `$${(((custom.amount_usd_cents ?? 0) * quantity) / 100).toFixed(2)}`
              }
              features={[
                <Feature key="c">
                  <strong className="font-semibold tabular-nums">
                    {creditsOf((custom.unit_credits_cents ?? 0) * quantity).toLocaleString()}
                  </strong>{" "}
                  <span className="text-muted-foreground">{t.credits}</span>
                </Feature>,
                <Feature key="e">
                  <span className="text-muted-foreground">{t.neverExpires}</span>
                </Feature>,
              ]}
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
          ) : null}
        </div>
      ) : null}

      {group === "redeem" ? (
        <div className="mx-auto mt-8 max-w-[600px]">
          <Card className="p-8 sm:p-10">
            <Label htmlFor="code" className="mb-3 text-sm text-muted-foreground">
              {t.activationCode}
            </Label>
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
              className="h-16 px-5 text-lg font-medium tracking-[0.12em] uppercase sm:text-xl sm:tracking-[0.14em]"
            />
            <Button
              size="lg"
              className="mt-5 h-14 w-full text-base"
              disabled={busy === "redeem"}
              onClick={() => void redeem()}
            >
              {t.redeem}
            </Button>
          </Card>
          <p className="mt-4 text-center text-sm leading-relaxed text-muted-foreground">{t.redeemNote}</p>
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
