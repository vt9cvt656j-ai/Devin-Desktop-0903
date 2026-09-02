import { useCallback, useEffect, useState } from "react";

import { Shell, navFor } from "@/components/Shell";
import { Paid } from "@/pages/Paid";
import { Billing } from "@/pages/Billing";
import { Dashboard, type Tab } from "@/pages/Dashboard";
import { api, claimPendingReferral, ensureSharedSession, type Catalog, type Me } from "@/lib/api";
import { planIsActive, planLabel, setCreditDivisor, setMoneyLocale } from "@/lib/format";
import { DICTS, LANGS, type Lang } from "@/lib/i18n";

const LANG_KEY = "mrday.lang";
const TABS: Tab[] = [
  "overview",
  "usage",
  "settings",
  "integrations",
  "devices",
  "models",
  "invite",
  "referrals",
  "settlements",
  "withdraw",
];

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
  /*
   * 从 Stripe 付款回来。success_url 是 /billing?paid=<session>，因为 nginx 里只有
   * /billing 和 /dashboard 两个写死的 location，新加路径要改 nginx，而 nginx 配置来自
   * 仓库、手改会被下次部署覆盖。
   *
   * 只读一次并立刻从地址栏抹掉：这个 session id 会被复制、被分享，留在历史里没有好处。
   * 抹掉之后组件不会重挂，所以 useState 的初始值就是这一次的值。
   */
  const [paidSession] = useState(() => {
    const sid = new URLSearchParams(location.search).get("paid");
    if (sid) history.replaceState(null, "", "/billing");
    return sid;
  });

  const [me, setMe] = useState<Me | null>(null);
  const [catalog, setCatalog] = useState<Catalog | null>(null);
  const [tab, setTab] = useState<Tab>(readTab);
  const [failed, setFailed] = useState(false);
  /*
   * 打款方式决定侧栏里有没有「提现」：开了自动打款就没有可提的动作，服务端也会拒绝
   * 手动申请。在根组件取一次 —— 侧栏每一页都在，放进去就是每切一页请求一遍同一个设置。
   */
  const [payoutNav, setPayoutNav] = useState({ batch: false, pending: 0 });
  /** 邀请码绑定的结果，只在这次加载里显示一次。 */
  const [refNote, setRefNote] = useState<{ text: string; ok: boolean } | null>(null);
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

  // Upgrade a pre-existing host-only session cookie so the marketing site can see it.
  useEffect(() => {
    ensureSharedSession();
  }, []);

  /*
   * Someone arrived through a referral link. Bind it now that there is a session.
   *
   * Here rather than in the gate because the gate is not on every path in: GitHub and
   * Google redirect from the provider straight to /dashboard and never run the gate's
   * completion code. This is the one place all three routes arrive at.
   *
   * 结果要说出来。
   *
   * 以前这里是完全静默的，理由是「拒绝的原因跟新用户没关系」。但实际发生的是：有人点了
   * 一个邀请链接，被送回后台，然后什么都没有 —— 绑上了没有、为什么没绑上，界面上一个字
   * 都没有，只能以为链接坏了。绑定成功要告诉他，被拒绝也要把服务端的原话摆出来
   * （「不能使用自己的邀请码」「邀请码只能在注册时使用」都是一句话就能解释清楚的事）。
   */
  useEffect(() => {
    if (!api.hasToken()) return;
    void claimPendingReferral().then((r) => {
      if (r.kind === "bound") setRefNote({ text: t.referralBound, ok: true });
      else if (r.kind === "refused" && r.message) setRefNote({ text: r.message, ok: false });
    });
    // 失败就按「显示」处理:少一个入口比多一个更难被发现。
    void api
      .referral()
      .then((r) => setPayoutNav({ batch: r.batch_enabled, pending: r.pending_withdrawals }))
      .catch(() => undefined);
  }, []);

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
  // 支付成功页不等目录：用户刚付完钱，等一份商品列表加载完才给他看结果没有道理。
  if (!me || (onBilling && !catalog && !paidSession)) {
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
      nav={navFor(payoutNav.batch, payoutNav.pending)}
    >
      {/*
        * 邀请码的结果。一次性、可关闭 —— 它回答的是「我刚点的那个链接怎么样了」，
        * 看过就没用了。
        */}
      {refNote && (
        <div
          role="status"
          className={`mb-4 flex items-start justify-between gap-3 rounded-lg border px-4 py-3 text-sm ${
            refNote.ok
              ? "border-emerald-200 bg-emerald-50 text-emerald-900 dark:border-emerald-900 dark:bg-emerald-950 dark:text-emerald-200"
              : "border-border bg-muted text-muted-foreground"
          }`}
        >
          <span>{refNote.text}</span>
          <button
            type="button"
            onClick={() => setRefNote(null)}
            className="shrink-0 opacity-60 transition-opacity hover:opacity-100"
            aria-label="close"
          >
            ×
          </button>
        </div>
      )}

      {paidSession ? (
        <Paid sessionId={paidSession} lang={lang} />
      ) : onBilling && catalog ? (
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
