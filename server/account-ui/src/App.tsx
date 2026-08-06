import { useCallback, useEffect, useState } from "react";

import { Shell } from "@/components/Shell";
import { Billing } from "@/pages/Billing";
import { Dashboard, type Tab } from "@/pages/Dashboard";
import { api, type Catalog, type Me } from "@/lib/api";
import { planIsActive, planLabel, setCreditDivisor, setMoneyLocale } from "@/lib/format";
import { DICTS, LANGS, type Lang } from "@/lib/i18n";

const LANG_KEY = "mrday.lang";
const TABS: Tab[] = ["overview", "usage", "settings", "integrations", "devices"];

/** Narrows an unknown stored or server value to a language this build actually has. */
function isLang(v: unknown): v is Lang {
  return typeof v === "string" && LANGS.some((l) => l.value === v);
}

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
  /*
   * Interface language.
   *
   * English is the default and the fallback: an unrecognised stored value (a locale that
   * was removed, or a corrupted key) falls back rather than rendering a blank console.
   * Held here, at the root, so switching it re-renders every page at once — the sidebar,
   * the tables and the dates all read the same `lang`.
   */
  const [lang, setLang] = useState<Lang>(() => {
    // Only the opening frame. The account's own choice arrives with /api/me a moment
    // later and wins; this is here so the first paint is not English for someone who
    // has already chosen otherwise on this machine.
    try {
      const saved = localStorage.getItem(LANG_KEY);
      if (isLang(saved)) return saved;
    } catch {
      /* storage can be blocked; English is the answer either way */
    }
    return "en";
  });

  /*
   * Written to the account, not just this browser.
   *
   * localStorage is updated first so the change is instant and survives a reload even
   * if the request fails; the account write is what makes the choice follow the person
   * to their next machine. A failed save is deliberately silent — the language has
   * already changed on screen, and an error toast about a preference would be noise.
   */
  function pickLang(next: Lang) {
    setLang(next);
    try {
      localStorage.setItem(LANG_KEY, next);
    } catch {
      /* a rejected write only means the choice is not remembered locally */
    }
    void api.updateProfile({ language: next }).catch(() => undefined);
  }

  const t = DICTS[lang];

  /*
   * Set during render, not in an effect.
   *
   * Money is formatted from a module-level locale, and an effect runs *after* the render
   * that already formatted it — and mutating module state does not schedule a re-render,
   * so nothing corrected it afterwards either. Prices were left one language behind:
   * switching English → German → Traditional Chinese printed "19,99 $" on a Chinese page.
   *
   * Doing it here is a side effect during render, which is normally worth avoiding. It is
   * safe in this one case because it is idempotent, depends only on `lang`, and App is the
   * root — so it has already run before any child formats a number.
   */
  setMoneyLocale(lang);

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
      // The account's language wins over whatever this browser remembered, so signing
      // in on a new machine brings your language with you.
      if (isLang(profile.language)) setLang(profile.language);
      if (cat) {
        setCreditDivisor(cat.raw_cents_per_credit_usd);
        setCatalog(cat);
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
    document.documentElement.lang = lang;
  }, [lang]);

  if (failed) {
    return <div className="p-16 text-center text-muted-foreground">{t.loadFailed}</div>;
  }
  if (!me || (onBilling && !catalog)) {
    return <div className="p-16 text-center text-muted-foreground">{t.loading}</div>;
  }

  const active = onBilling ? "/billing" : `/dashboard#${tab}`;
  const planText = planIsActive(me.plan, me.plan_expires_at) ? planLabel(me.plan, lang) : "Free";

  return (
    <Shell
      lang={lang}
      active={active}
      email={me.email}
      // US order, and tolerant of only one half being filled in.
      name={[me.first_name, me.last_name].filter(Boolean).join(" ").trim()}
      avatar={me.avatar}
      planLabel={planText}
      onLangChange={pickLang}
    >
      {onBilling && catalog ? (
        <Billing
          catalog={catalog}
          me={me}
          lang={lang}
          onRedeemed={() => void load()}
        />
      ) : (
        <Dashboard
          me={me}
          tab={tab}
          lang={lang}
          catalog={catalog}
          onProfileSaved={() => void load()}
          onLangChange={pickLang}
        />
      )}
    </Shell>
  );
}
