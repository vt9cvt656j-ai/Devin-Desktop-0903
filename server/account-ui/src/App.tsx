import { useCallback, useEffect, useState } from "react";

import { Shell } from "@/components/Shell";
import { Billing } from "@/pages/Billing";
import { Dashboard, type Tab } from "@/pages/Dashboard";
import { api, type Catalog, type Me } from "@/lib/api";
import { planIsActive, planLabel, setCreditDivisor } from "@/lib/format";
import { DICTS, type Currency, type Lang } from "@/lib/i18n";
import { cn } from "@/lib/utils";

const TABS: Tab[] = ["overview", "usage", "settings", "integrations"];
const CURRENCY_KEY = "mrday.currency";

function readTab(): Tab {
  const h = location.hash.replace("#", "") as Tab;
  return TABS.includes(h) ? h : "overview";
}

export default function App() {
  const onBilling = location.pathname.startsWith("/billing");

  const [me, setMe] = useState<Me | null>(null);
  const [catalog, setCatalog] = useState<Catalog | null>(null);
  const [tab, setTab] = useState<Tab>(readTab);
  const [failed, setFailed] = useState(false);
  const [currency, setCurrency] = useState<Currency>("usd");
  /** True while the currency is the one the gateway inferred, not one the user picked. */
  const [autoCurrency, setAutoCurrency] = useState(true);

  // Language follows the currency: someone paying in yuan is reading Chinese.
  const lang: Lang = currency === "cny" ? "zh" : "en";
  const t = DICTS[lang];

  const load = useCallback(async () => {
    try {
      const [profile, cat] = await Promise.all([
        api.me(),
        // The overview needs it too, to price the plan the account is on. Failing to
        // load the catalogue must not take the dashboard down with it — it is extra
        // detail on one card there, whereas /billing genuinely cannot render without it.
        api.catalog().catch(() => null),
      ]);
      setCreditDivisor(profile.raw_cents_per_credit_usd);
      setMe(profile);
      if (cat) {
        setCreditDivisor(cat.raw_cents_per_credit_usd);
        setCatalog(cat);
        // Currency still gets decided on the billing page alone. It also selects the
        // interface language, so adopting the geo-inferred default here would swing a
        // CN visitor's whole dashboard into Chinese without them asking for it.
        if (onBilling) {
          let saved: string | null = null;
          try {
            saved = localStorage.getItem(CURRENCY_KEY);
          } catch {
            /* storage may be blocked */
          }
          if (saved === "cny" || saved === "usd") {
            setCurrency(saved);
            setAutoCurrency(false);
          } else {
            setCurrency(cat.currency === "cny" ? "cny" : "usd");
            setAutoCurrency(true);
          }
        }
      }
    } catch {
      setFailed(true);
    }
  }, [onBilling]);

  useEffect(() => {
    if (!api.hasToken()) {
      location.replace(`/gate?next=${encodeURIComponent(location.pathname)}`);
      return;
    }
    void load();
  }, [load]);

  useEffect(() => {
    const onHash = () => setTab(readTab());
    window.addEventListener("hashchange", onHash);
    return () => window.removeEventListener("hashchange", onHash);
  }, []);

  useEffect(() => {
    document.documentElement.lang = lang === "zh" ? "zh-CN" : "en";
  }, [lang]);

  function pickCurrency(c: Currency) {
    setCurrency(c);
    setAutoCurrency(false);
    try {
      localStorage.setItem(CURRENCY_KEY, c);
    } catch {
      /* a rejected write only means the choice is not remembered */
    }
  }

  /*
   * Which currency to price the plan in — a different question from which language to
   * render, even though `currency` state answers both on the billing page.
   *
   * Money should read the way the account actually transacts: a buyer charged in yuan
   * wants to see ¥488, not a dollar figure they never paid. Language should not be
   * dragged along with it — deriving `lang` from this would swing the whole dashboard
   * into Chinese for anyone billed in CNY, which is not what "show me the price I paid"
   * asked for. So this reads the buyer's saved choice, falls back to the country the
   * gateway inferred, and stops there.
   */
  const priceCurrency: Currency = (() => {
    let saved: string | null = null;
    try {
      saved = localStorage.getItem(CURRENCY_KEY);
    } catch {
      /* storage may be blocked; the inferred default still applies */
    }
    if (saved === "cny" || saved === "usd") return saved;
    return catalog?.currency === "cny" ? "cny" : "usd";
  })();

  if (failed) {
    return <div className="p-16 text-center text-muted-foreground">{t.loadFailed}</div>;
  }
  if (!me || (onBilling && !catalog)) {
    return <div className="p-16 text-center text-muted-foreground">{t.loading}</div>;
  }

  const active = onBilling ? "/billing" : `/dashboard#${tab}`;
  const planText = planIsActive(me.plan, me.plan_expires_at) ? planLabel(me.plan) : "Free";

  return (
    <Shell
      lang={lang}
      active={active}
      email={me.email}
      planLabel={planText}
      footer={
        onBilling ? (
          <div className="flex items-center justify-between gap-2 px-1">
            <span className="text-[11px] text-muted-foreground">
              {autoCurrency
                ? `${t.autoCurrency}${catalog?.country ? ` (${catalog.country})` : ""}`
                : t.manualCurrency}
            </span>
            <div className="flex gap-0.5 rounded-lg bg-muted p-0.5">
              {(["cny", "usd"] as const).map((c) => (
                <button
                  key={c}
                  type="button"
                  onClick={() => pickCurrency(c)}
                  className={cn(
                    "rounded-md px-2.5 py-1 text-[11.5px] font-semibold transition-colors",
                    currency === c ? "bg-card text-foreground" : "text-muted-foreground hover:text-foreground",
                  )}
                >
                  {c.toUpperCase()}
                </button>
              ))}
            </div>
          </div>
        ) : null
      }
    >
      {onBilling && catalog ? (
        <Billing
          catalog={catalog}
          me={me}
          lang={lang}
          currency={currency}
          onRedeemed={() => void load()}
        />
      ) : (
        <Dashboard me={me} tab={tab} lang={lang} catalog={catalog} currency={priceCurrency} />
      )}
    </Shell>
  );
}
