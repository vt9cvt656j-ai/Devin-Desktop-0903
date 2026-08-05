import { useEffect, useRef, useState } from "react";
import { Camera, ChevronLeft, ChevronRight } from "lucide-react";

import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { BRAND_MARKS } from "@/components/BrandMarks";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  api,
  probeDesktop,
  signOut,
  type Catalog,
  type DesktopProbe,
  type Integration,
  type Me,
  type Usage,
} from "@/lib/api";
import {
  formatDate,
  formatDateTime,
  planIsActive,
  planLabel,
  price,
  timeUntil,
  usd,
} from "@/lib/format";
import { DICTS, type Currency, type Lang } from "@/lib/i18n";
import { ACCEPTED, AvatarError, fileToAvatarDataUrl } from "@/lib/avatar";
import { cn } from "@/lib/utils";

/*
 * The release repo is private, so github.com/.../releases/latest 404s for every signed-in
 * user who clicks Download here. The marketing site's download section reads the gateway's
 * public update feed and shows either a real installer link or an honest "not published
 * yet" — either beats a 404.
 */
const RELEASES = "https://www.michaelide.xyz/#download";

export type Tab = "overview" | "usage" | "settings" | "integrations";

/**
 * Centred, not left-aligned. These sit in a row of three: ragged left-aligned blocks of
 * different lengths read as misaligned rather than as a set. Centring is applied here so
 * the Overview row and the Usage row cannot drift apart.
 */
function Stat({ label, value, sub }: { label: string; value: React.ReactNode; sub?: string }) {
  return (
    <Card className="items-center bg-muted p-6 text-center">
      <p className="text-xs text-muted-foreground">{label}</p>
      <div className="mt-1.5 text-[22px] font-semibold tracking-tight tabular-nums">{value}</div>
      {sub ? <div className="mt-1.5 text-xs text-muted-foreground">{sub}</div> : null}
    </Card>
  );
}

function Row({ k, v }: { k: string; v: React.ReactNode }) {
  return (
    <>
      <div className="flex items-baseline justify-between gap-5 py-3">
        <span className="text-[13.5px] text-muted-foreground">{k}</span>
        <span className="break-all text-right font-medium">{v}</span>
      </div>
      <Separator className="last:hidden" />
    </>
  );
}

/**
 * Name and picture, edited in place.
 *
 * The picture saves the moment one is chosen rather than waiting for the button: a
 * preview that is not yet stored is a trap, because the obvious next move after seeing
 * your face appear is to navigate away. The name fields do wait for Save — text is
 * edited character by character and saving each keystroke would be absurd.
 */
function ProfileCard({
  me,
  lang,
  onSaved,
}: {
  me: Me;
  lang: Lang;
  onSaved: () => void;
}) {
  const t = DICTS[lang];
  const [first, setFirst] = useState(me.first_name ?? "");
  const [last, setLast] = useState(me.last_name ?? "");
  const [avatar, setAvatar] = useState<string | null>(me.avatar ?? null);
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState<{ text: string; ok: boolean } | null>(null);
  const fileRef = useRef<HTMLInputElement | null>(null);

  const dirty = first !== (me.first_name ?? "") || last !== (me.last_name ?? "");

  async function pick(file: File | undefined) {
    if (!file) return;
    setNote(null);
    try {
      const dataUrl = await fileToAvatarDataUrl(file);
      setBusy(true);
      await api.updateProfile({ avatar: dataUrl });
      setAvatar(dataUrl);
      setNote({ text: t.profileSaved, ok: true });
      onSaved();
    } catch (e) {
      const kind = e instanceof AvatarError ? e.message : "";
      setNote({
        text: kind === "too-large" ? t.pictureTooLarge : t.pictureUnreadable,
        ok: false,
      });
    } finally {
      setBusy(false);
      // Let the same file be chosen again after a failure.
      if (fileRef.current) fileRef.current.value = "";
    }
  }

  async function remove() {
    setBusy(true);
    setNote(null);
    try {
      await api.updateProfile({ avatar: "" });
      setAvatar(null);
      setNote({ text: t.profileSaved, ok: true });
      onSaved();
    } catch {
      setNote({ text: t.pictureUnreadable, ok: false });
    } finally {
      setBusy(false);
    }
  }

  async function save() {
    setBusy(true);
    setNote(null);
    try {
      await api.updateProfile({ first_name: first.trim(), last_name: last.trim() });
      setFirst(first.trim());
      setLast(last.trim());
      setNote({ text: t.profileSaved, ok: true });
      onSaved();
    } catch (e) {
      setNote({ text: e instanceof Error ? e.message : "…", ok: false });
    } finally {
      setBusy(false);
    }
  }

  const initial = (first || me.email || "?").charAt(0).toUpperCase();

  return (
    <Card className="bg-muted p-6">
      <div className="flex flex-col gap-6 sm:flex-row sm:items-start">
        {/*
          * The picture is the control. A separate "Change" button beside it left two
          * buttons sitting at different heights against a column of inputs, and made
          * the obvious target — the picture itself — inert. Clicking it opens the file
          * dialog; hovering (or tabbing to it) says so.
          */}
        <div className="flex shrink-0 flex-col items-center gap-2">
          <button
            type="button"
            onClick={() => fileRef.current?.click()}
            disabled={busy}
            aria-label={t.changePicture}
            className="group relative size-20 rounded-full outline-none ring-offset-background transition-opacity focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:opacity-60"
          >
            <Avatar className="size-20">
              {avatar ? <AvatarImage src={avatar} alt="" /> : null}
              <AvatarFallback className="bg-primary text-xl font-semibold text-primary-foreground">
                {initial}
              </AvatarFallback>
            </Avatar>
            {/* Fixed black-and-white, not theme tokens. The fallback avatar is already
                `bg-primary` — near black — so a `foreground/60` scrim over it was black
                on black and the label all but vanished. This has to stay legible over a
                dark fallback, a light photo and a dark one alike. */}
            <span className="pointer-events-none absolute inset-0 flex flex-col items-center justify-center gap-1 rounded-full bg-black/70 text-[10px] font-semibold uppercase tracking-wider text-white opacity-0 transition-opacity duration-150 group-hover:opacity-100 group-focus-visible:opacity-100">
              <Camera className="size-4" />
              {t.changePicture}
            </span>
          </button>
          {avatar ? (
            <Button
              variant="ghost"
              size="sm"
              className="h-7 px-2 text-xs text-muted-foreground"
              disabled={busy}
              onClick={() => void remove()}
            >
              {t.removePicture}
            </Button>
          ) : null}
          <input
            ref={fileRef}
            type="file"
            accept={ACCEPTED}
            className="hidden"
            onChange={(e) => void pick(e.target.files?.[0])}
          />
        </div>

        <div className="min-w-0 flex-1">
          <p className="mb-4 text-[13.5px] leading-relaxed text-muted-foreground">
            {t.profileNote} {t.pictureHint}
          </p>
          {/* US order: given name first. */}
          <div className="grid gap-4 sm:grid-cols-2">
            <div>
              <Label htmlFor="first-name" className="mb-1.5 text-xs text-muted-foreground">
                {t.firstName}
              </Label>
              <Input
                id="first-name"
                value={first}
                autoComplete="given-name"
                maxLength={64}
                onChange={(e) => setFirst(e.target.value)}
              />
            </div>
            <div>
              <Label htmlFor="last-name" className="mb-1.5 text-xs text-muted-foreground">
                {t.lastName}
              </Label>
              <Input
                id="last-name"
                value={last}
                autoComplete="family-name"
                maxLength={64}
                onChange={(e) => setLast(e.target.value)}
              />
            </div>
          </div>
          <div className="mt-4 flex items-center gap-3">
            <Button disabled={busy || !dirty} onClick={() => void save()}>
              {busy ? t.saving : t.saveProfile}
            </Button>
            {note ? (
              <span
                className={cn("text-[13px]", note.ok ? "text-success" : "text-destructive")}
              >
                {note.text}
              </span>
            ) : null}
          </div>
        </div>
      </div>
    </Card>
  );
}

/**
 * Linked code hosts.
 *
 * Providers the server has no credentials for are shown greyed out rather than hidden —
 * "GitHub, not available yet" tells you where you stand; a missing row reads as a
 * product that never had the feature.
 */
function CodeHosts({ lang }: { lang: Lang }) {
  const t = DICTS[lang];
  const [rows, setRows] = useState<Integration[] | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [note, setNote] = useState<{ text: string; ok: boolean } | null>(null);
  /** Which card is currently showing its token box, and what has been typed into it. */
  const [tokenFor, setTokenFor] = useState<string | null>(null);
  const [token, setToken] = useState("");

  const load = () => {
    void api
      .integrations()
      .then((r) => setRows(r.providers))
      .catch(() => setRows([]));
  };
  useEffect(load, []);

  // The OAuth callback lands back here with ?integration=… — report it, then take it out
  // of the URL so a refresh does not replay a stale message.
  useEffect(() => {
    const outcome = new URLSearchParams(location.search).get("integration");
    if (!outcome) return;
    setNote(
      outcome === "cancelled"
        ? { text: t.integrationCancelled, ok: false }
        : outcome === "error"
          ? { text: t.integrationError, ok: false }
          : { text: t.integrationConnected, ok: true },
    );
    const url = new URL(location.href);
    url.searchParams.delete("integration");
    history.replaceState(null, "", `${url.pathname}${url.search}${url.hash}`);
  }, [t]);

  async function connect(provider: string) {
    setBusy(provider);
    setNote(null);
    try {
      const { url } = await api.integrationStart(provider);
      location.href = url;
    } catch (e) {
      setNote({ text: e instanceof Error ? e.message : t.integrationError, ok: false });
      setBusy(null);
    }
  }

  async function saveToken(provider: string) {
    const value = token.trim();
    if (!value) return;
    setBusy(provider);
    setNote(null);
    try {
      const res = await api.integrationConnectToken(provider, value);
      setTokenFor(null);
      // Cleared rather than left in state: there is no reason to keep a live token in
      // memory once it has been handed over.
      setToken("");
      setNote({ text: `${t.integrationConnected} (${res.account_login})`, ok: true });
      load();
    } catch (e) {
      setNote({ text: e instanceof Error ? e.message : t.integrationError, ok: false });
    } finally {
      setBusy(null);
    }
  }

  async function disconnect(provider: string) {
    setBusy(provider);
    setNote(null);
    try {
      const res = await api.integrationDisconnect(provider);
      setNote({ text: `${t.disconnectedNote} ${res.revoke_at_provider}`, ok: true });
      load();
    } catch (e) {
      setNote({ text: e instanceof Error ? e.message : t.integrationError, ok: false });
    } finally {
      setBusy(null);
    }
  }

  return (
    <>
      <h2 className="mb-1 mt-8 text-sm font-semibold">{t.codeHosts}</h2>
      <p className="mb-3 text-[13.5px] text-muted-foreground">{t.codeHostsLede}</p>
      {note ? (
        <p
          className={cn(
            "mb-3 break-all text-[13px]",
            note.ok ? "text-success" : "text-destructive",
          )}
        >
          {note.text}
        </p>
      ) : null}
      <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
        {(rows ?? []).map((row) => {
          const Mark = BRAND_MARKS[row.provider];
          const open = tokenFor === row.provider;
          return (
            <Card key={row.provider} className="items-center bg-muted p-6 text-center">
              <div className="mb-2.5 flex items-center justify-center gap-2.5">
                {Mark ? <Mark className="size-5" /> : null}
                <span className="text-lg font-semibold">{row.label}</span>
                {row.connected ? <Badge variant="success">{t.connected}</Badge> : null}
              </div>
              <p className="mb-4 min-h-[2.5rem] text-[13.5px] leading-relaxed text-muted-foreground">
                {row.connected
                  ? `${t.connectedAs} ${row.account_login || row.account_name || "—"}`
                  : `@${row.provider}:`}
              </p>

              {row.connected ? (
                <Button
                  variant="outline"
                  disabled={busy === row.provider}
                  onClick={() => void disconnect(row.provider)}
                >
                  {t.disconnect}
                </Button>
              ) : open ? (
                // Paste path. Type=password so a token does not sit in plain sight on a
                // shared screen; the browser is told not to remember it either.
                <div className="flex w-full max-w-xs flex-col items-center gap-2.5">
                  <Input
                    autoFocus
                    type="password"
                    autoComplete="off"
                    spellCheck={false}
                    placeholder={t.tokenPlaceholder}
                    value={token}
                    onChange={(e) => setToken(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") void saveToken(row.provider);
                    }}
                  />
                  <p className="text-xs leading-relaxed text-muted-foreground">
                    {t.tokenScopes} <code className="font-medium">{row.token_hint}</code>
                    {" · "}
                    <a
                      className="underline underline-offset-2"
                      href={row.token_url}
                      target="_blank"
                      rel="noreferrer"
                    >
                      {t.tokenCreate}
                    </a>
                  </p>
                  <div className="flex gap-2">
                    <Button
                      disabled={!token.trim() || busy === row.provider}
                      onClick={() => void saveToken(row.provider)}
                    >
                      {busy === row.provider ? t.connecting : t.connect}
                    </Button>
                    <Button
                      variant="ghost"
                      disabled={busy === row.provider}
                      onClick={() => {
                        setTokenFor(null);
                        setToken("");
                      }}
                    >
                      {t.cancel}
                    </Button>
                  </div>
                </div>
              ) : (
                <div className="flex flex-col items-center gap-2">
                  <div className="flex flex-wrap items-center justify-center gap-2">
                    {/* Only when an OAuth app exists. Its absence no longer disables the
                        card — the token route below always works. */}
                    {row.oauth_configured ? (
                      <Button
                        disabled={busy === row.provider}
                        onClick={() => void connect(row.provider)}
                      >
                        {busy === row.provider ? t.connecting : t.connect}
                      </Button>
                    ) : null}
                    {/* Named for what it does. Labelled plain "Connect" it promised a
                        jump to GitHub's sign-in page and instead opened a paste box,
                        which reads as a broken button rather than a different route. */}
                    <Button
                      variant={row.oauth_configured ? "outline" : "default"}
                      onClick={() => {
                        setNote(null);
                        setToken("");
                        setTokenFor(row.provider);
                      }}
                    >
                      {row.oauth_configured ? t.useToken : t.connectWithToken}
                    </Button>
                  </div>
                  {!row.oauth_configured ? (
                    <p className="text-xs leading-relaxed text-muted-foreground">
                      {t.oauthUnavailable}
                    </p>
                  ) : null}
                </div>
              )}
            </Card>
          );
        })}
      </div>
    </>
  );
}

/** Rows per page on the Usage tab. The gateway hands us the most recent 200. */
const USAGE_PAGE_SIZE = 20;

/**
 * Which page numbers to render: always the first and last, the current one and its
 * neighbours, and an ellipsis standing in for whatever is skipped. At 200 rows this
 * never needs to skip anything, but the list is not guaranteed to stay that size.
 */
function pageWindow(current: number, count: number): (number | "gap")[] {
  if (count <= 9) return Array.from({ length: count }, (_, i) => i + 1);
  const out: (number | "gap")[] = [1];
  const from = Math.max(2, current - 2);
  const to = Math.min(count - 1, current + 2);
  if (from > 2) out.push("gap");
  for (let p = from; p <= to; p += 1) out.push(p);
  if (to < count - 1) out.push("gap");
  out.push(count);
  return out;
}

/** Fill {name} placeholders — the dictionary holds strings, not functions. */
function fill(template: string, values: Record<string, string | number>): string {
  return template.replace(/\{(\w+)\}/g, (_, key) => String(values[key] ?? ""));
}

function UsageTable({
  rows,
  lang,
  limit,
  pageSize,
}: {
  rows: Usage["recent"];
  lang: Lang;
  /** Show at most this many rows and stop. Ignored when pageSize is given. */
  limit?: number;
  /** Paginate at this many rows per page instead of truncating. */
  pageSize?: number;
}) {
  const t = DICTS[lang];
  const [page, setPage] = useState(1);
  const top = useRef<HTMLDivElement | null>(null);
  const painted = useRef(false);

  const total = rows.length;
  const pageCount = pageSize ? Math.max(1, Math.ceil(total / pageSize)) : 1;
  // Usage refetches. If the list comes back shorter, don't strand the reader on a page
  // past the end — derive the page rather than trusting the stored number.
  const current = Math.min(page, pageCount);

  // Put the top of the table back in view after a page change, but never on first
  // paint — that would yank the page around on load.
  useEffect(() => {
    if (!painted.current) {
      painted.current = true;
      return;
    }
    top.current?.scrollIntoView({ block: "nearest", behavior: "smooth" });
  }, [current]);

  if (!rows.length) return <p className="py-6 text-center text-[13.5px] text-muted-foreground">{t.noRequests}</p>;

  const start = pageSize ? (current - 1) * pageSize : 0;
  const visible = pageSize ? rows.slice(start, start + pageSize) : rows.slice(0, limit ?? total);

  return (
    <div ref={top}>
    <div className="overflow-x-auto">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>{t.when}</TableHead>
            <TableHead>{t.model}</TableHead>
            <TableHead className="text-right">{t.tokensIn}</TableHead>
            <TableHead className="text-right">{t.tokensOut}</TableHead>
            <TableHead className="text-right">{t.cost}</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {visible.map((r, i) => (
            <TableRow key={`${r.time}-${i}`}>
              <TableCell className="whitespace-nowrap">{formatDateTime(r.time, lang)}</TableCell>
              <TableCell className="font-medium">
                {r.model}
                {r.estimated ? (
                  <Badge variant="outline" className="ml-2">
                    {t.estimated}
                  </Badge>
                ) : null}
              </TableCell>
              <TableCell className="text-right tabular-nums">
                {r.prompt_tokens == null ? "—" : r.prompt_tokens.toLocaleString()}
              </TableCell>
              <TableCell className="text-right tabular-nums">
                {r.completion_tokens == null ? "—" : r.completion_tokens.toLocaleString()}
              </TableCell>
              <TableCell className="text-right tabular-nums">
                {r.free_points_spent > 0
                  ? `${Math.round(r.free_points_spent * 1000) / 1000} ${t.credits}`
                  : usd(r.cost_cents, 4)}
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>

      {pageSize && pageCount > 1 ? (
        <div className="mt-4 flex flex-col-reverse items-center justify-between gap-3 sm:flex-row">
          <p className="text-xs tabular-nums text-muted-foreground">
            {fill(t.showingRange, {
              from: start + 1,
              to: start + visible.length,
              total,
            })}
          </p>
          <nav className="flex items-center gap-1" aria-label={t.requests}>
            <Button
              variant="outline"
              size="sm"
              className="px-2.5"
              disabled={current === 1}
              onClick={() => setPage(current - 1)}
              aria-label={t.pagePrev}
            >
              <ChevronLeft />
              <span className="hidden sm:inline">{t.pagePrev}</span>
            </Button>

            {/* Numbered jumps need room. Below sm they would wrap onto a second line,
                so the phone gets a plain position counter between the arrows instead. */}
            <div className="hidden items-center gap-1 sm:flex">
              {pageWindow(current, pageCount).map((p, i) =>
                p === "gap" ? (
                  <span key={`gap-${i}`} aria-hidden className="px-1 text-sm text-muted-foreground">
                    …
                  </span>
                ) : (
                  <Button
                    key={p}
                    size="sm"
                    variant={p === current ? "default" : "ghost"}
                    className="w-9 px-0 tabular-nums"
                    aria-current={p === current ? "page" : undefined}
                    aria-label={fill(t.goToPage, { page: p })}
                    onClick={() => setPage(p)}
                  >
                    {p}
                  </Button>
                ),
              )}
            </div>
            <span className="px-2 text-sm tabular-nums text-muted-foreground sm:hidden">
              {current} / {pageCount}
            </span>

            <Button
              variant="outline"
              size="sm"
              className="px-2.5"
              disabled={current === pageCount}
              onClick={() => setPage(current + 1)}
              aria-label={t.pageNext}
            >
              <span className="hidden sm:inline">{t.pageNext}</span>
              <ChevronRight />
            </Button>
          </nav>
        </div>
      ) : null}
    </div>
  );
}

export function Dashboard({
  me,
  tab,
  lang,
  catalog,
  currency,
  onProfileSaved,
}: {
  me: Me;
  tab: Tab;
  lang: Lang;
  /** null when the catalogue could not be loaded; the plan card just omits the price. */
  catalog: Catalog | null;
  currency: Currency;
  /** Re-reads the profile so the sidebar picks up a new name or picture immediately. */
  onProfileSaved: () => void;
}) {
  const t = DICTS[lang];
  const [usage, setUsage] = useState<Usage | null>(null);
  const [desktop, setDesktop] = useState<DesktopProbe | undefined>(undefined);
  const [checkingDesktop, setCheckingDesktop] = useState(false);
  const [modelCount, setModelCount] = useState<number | null>(null);

  useEffect(() => {
    // Additive: a failure here leaves the page usable.
    void api.usage().then(setUsage).catch(() => undefined);
    void api.models().then((m) => setModelCount(Array.isArray(m) ? m.length : null)).catch(() => undefined);
    void probeDesktop().then(setDesktop);
  }, []);

  /** Runs inside the click, which is the only context Chrome will prompt from. */
  async function recheckDesktop() {
    setCheckingDesktop(true);
    try {
      setDesktop(await probeDesktop());
    } finally {
      setCheckingDesktop(false);
    }
  }

  const cap = me.quota_window_cap_cents ?? 0;
  const left = me.quota_window_cents ?? 0;
  const spent = Math.max(0, cap - left);
  const pct = cap > 0 ? Math.max(0, Math.min(100, (spent / cap) * 100)) : null;
  const refill = timeUntil(me.quota_window_reset_at);
  const active = planIsActive(me.plan, me.plan_expires_at);
  // One catalogue row per plan, so this is unambiguous. Free accounts match nothing.
  const planPrice =
    active && me.plan
      ? (catalog?.items.find((i) => i.kind === "plan" && i.plan === me.plan) ?? null)
      : null;

  if (tab === "usage") {
    return (
      <div className="max-w-[1080px]">
        <h1 className="text-xl font-semibold tracking-tight">{t.usage}</h1>
        <p className="mb-6 mt-0.5 text-[13.5px] text-muted-foreground">{t.usageLede}</p>
        <div className="grid grid-cols-1 gap-4 md:grid-cols-3">
          <Stat label={t.spentAllTime} value={usage ? usd(usage.total_spent_cents) : "—"} />
          <Stat
            label={t.requestsShown}
            value={usage ? usage.recent.length : "—"}
            sub={t.requestsShownSub}
          />
          <Stat label={t.creditBalance} value={usage ? usd(usage.credits_cents) : "—"} />
        </div>
        <h2 className="mb-3 mt-8 text-sm font-semibold">{t.requests}</h2>
        <UsageTable rows={usage?.recent ?? []} lang={lang} pageSize={USAGE_PAGE_SIZE} />
      </div>
    );
  }

  if (tab === "settings") {
    return (
      <div className="max-w-[1080px]">
        <h1 className="text-xl font-semibold tracking-tight">{t.settings}</h1>
        <p className="mb-6 mt-0.5 text-[13.5px] text-muted-foreground">{t.settingsLede}</p>

        <h2 className="mb-3 text-sm font-semibold">{t.profile}</h2>
        <ProfileCard me={me} lang={lang} onSaved={onProfileSaved} />

        <h2 className="mb-3 text-sm font-semibold">{t.account}</h2>
        <Card className="bg-muted px-6 py-1">
          <Row k={t.email} v={me.email} />
          <Row k={t.accountId} v={me.id} />
          <Row k={t.role} v={me.role === "admin" ? t.administrator : t.member} />
          <Row k={t.memberSince} v={formatDate(me.created_at, lang)} />
          <Row k={t.lastSignIn} v={formatDateTime(me.last_login_at, lang)} />
        </Card>

        <h2 className="mb-3 mt-8 text-sm font-semibold">{t.plan}</h2>
        <Card className="bg-muted px-6 py-1">
          <Row k={t.currentPlan} v={planLabel(me.plan)} />
          <Row k={t.expires} v={me.plan_expires_at ? formatDate(me.plan_expires_at, lang) : "—"} />
          <Row k={t.includedQuota} v={cap > 0 ? `${usd(cap)} ${t.perWindow}` : t.notIncluded} />
          <Row
            k={t.weeklyCap}
            v={me.quota_weekly_cap_cents > 0 ? `${usd(me.quota_weekly_cap_cents)} ${t.perWeek}` : t.noWeeklyCap}
          />
        </Card>

        <h2 className="mb-3 mt-8 text-sm font-semibold">{t.session}</h2>
        <Card className="bg-muted p-6">
          <p className="mb-4 text-[13.5px] leading-relaxed text-muted-foreground">{t.signOutNote}</p>
          <Button variant="outline" onClick={signOut} className="w-fit">
            {t.signOut}
          </Button>
        </Card>
      </div>
    );
  }

  if (tab === "integrations") {
    return (
      <div className="max-w-[1080px]">
        <h1 className="text-xl font-semibold tracking-tight">{t.integrations}</h1>
        <p className="mb-6 mt-0.5 text-[13.5px] text-muted-foreground">{t.integrationsLede}</p>

        <h2 className="mb-3 text-sm font-semibold">{t.desktopApp}</h2>
        <Card className="bg-muted p-6">
          <div className="mb-2.5 flex items-baseline gap-2.5">
            <span className="text-lg font-semibold">{t.desktopApp}</span>
            {desktop === undefined ? null : desktop.state === "connected" ? (
              desktop.session.signedIn ? (
                <Badge variant="success">{t.connected}</Badge>
              ) : (
                <Badge variant="outline">{t.signedOut}</Badge>
              )
            ) : desktop.state === "needs-permission" ? (
              <Badge variant="outline">{t.desktopNeedsPermission}</Badge>
            ) : desktop.state === "permission-blocked" ? (
              <Badge variant="outline">{t.desktopPermissionBlocked}</Badge>
            ) : (
              <Badge variant="outline">{t.notDetected}</Badge>
            )}
          </div>
          <p className="mb-4 text-[13.5px] leading-relaxed text-muted-foreground">
            {desktop === undefined
              ? t.loading
              : desktop.state === "connected"
                ? desktop.session.viaServer
                  // Reported by the gateway: it knows an app is signed in to this
                  // account, not which machine it is on, so the wording does not claim
                  // "here".
                  ? `${t.desktopOnline} ${desktop.session.secondsAgo ?? 0} ${t.desktopSecondsAgo}${desktop.session.version ? ` (${t.desktopVersion} ${desktop.session.version})` : ""}`
                  : desktop.session.signedIn
                    ? `${t.desktopConnected} ${desktop.session.email} (${t.desktopVersion} ${desktop.session.version}). ${t.desktopReuse}`
                    : t.desktopSignedOut
                : desktop.state === "needs-permission"
                  ? t.desktopPermissionAsk
                  : desktop.state === "permission-blocked"
                    ? t.desktopPermissionBlockedHelp
                    : t.desktopUnreachable}
          </p>
          {/* The observed facts, verbatim. Without these the page was asserting a cause
              it could not know, and "the app is not running" was flatly wrong while the
              app was running, listening, and answering the preflight correctly. */}
          {desktop !== undefined && desktop.state === "unreachable" ? (
            <p className="mb-4 font-mono text-[11.5px] leading-relaxed text-muted-foreground">
              {t.desktopDetail}: local-network-access = {desktop.permission}
              {desktop.error ? ` · ${desktop.error}` : null}
            </p>
          ) : null}
          <div className="flex flex-wrap items-center gap-2.5">
            {/* Chrome raises the local-network prompt only off a real click, so this
                button exists to be that click. Offered whenever we are not already
                connected — after a block it is also how you re-test once the setting
                has been changed back. */}
            {desktop !== undefined && desktop.state !== "connected" ? (
              <Button className="w-fit" disabled={checkingDesktop} onClick={() => void recheckDesktop()}>
                {checkingDesktop ? t.desktopChecking : t.desktopConnectButton}
              </Button>
            ) : null}
            <Button variant="outline" asChild className="w-fit">
              <a href={RELEASES} target="_blank" rel="noreferrer">
                {t.download}
              </a>
            </Button>
          </div>
        </Card>

        <CodeHosts lang={lang} />

        <h2 className="mb-3 mt-8 text-sm font-semibold">{t.apiHeading}</h2>
        <Card className="bg-muted px-6 py-1">
          <Row k={t.baseUrl} v={location.origin} />
          <Row k={t.auth} v={t.authValue} />
          <Row k={t.modelsAvailable} v={modelCount == null ? "—" : `${modelCount} ${t.available}`} />
        </Card>
      </div>
    );
  }

  return (
    <div className="max-w-[1080px]">
      <h1 className="text-xl font-semibold tracking-tight">{t.overview}</h1>
      <p className="mb-6 mt-0.5 text-[13.5px] text-muted-foreground">{me.email}</p>

      <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
        <Card className="bg-muted p-6">
          <p className="mb-3.5 text-[13.5px] text-muted-foreground">{t.includedUsage}</p>
          {pct === null ? (
            <>
              <p className="mb-4 text-3xl font-semibold tracking-tight">
                {t.noneIncluded}{" "}
                <span className="text-[15px] font-medium text-muted-foreground">{t.onFreePlan}</span>
              </p>
              <p className="text-xs text-muted-foreground">{t.freeFallback}</p>
            </>
          ) : (
            <>
              {/* The amount spent used to sit here as "$0.00 of $45.25", which restated the
                  percentage beside it — two ways of saying nothing was used. Only the
                  headline keeps the reading; the allowance moves to the right, where it
                  labels the far end of the bar it belongs to. */}
              {/* flex-wrap so a long pair ("100% 已使用" + "$120.66 每时段") drops onto a
                  second line on a phone instead of crushing the headline. */}
              <div className="mb-4 flex flex-wrap items-baseline justify-between gap-x-3 gap-y-1">
                <p className="text-3xl font-semibold tracking-tight tabular-nums">
                  {Math.round(pct)}% {t.used}
                </p>
                <p className="shrink-0 text-[15px] font-medium tabular-nums text-muted-foreground">
                  {usd(cap)} {t.perWindow}
                </p>
              </div>
              <div className="h-1 overflow-hidden rounded-full bg-accent">
                <div className="h-full rounded-full bg-foreground transition-all" style={{ width: `${pct}%` }} />
              </div>
              <p className="mt-3.5 text-xs text-muted-foreground">
                {refill.expired ? t.refillsNow : `${t.refillsIn} ${refill.text}`}
              </p>
            </>
          )}
        </Card>

        <Card className="items-center bg-muted p-6 text-center">
          <div className="mb-2.5 flex items-baseline justify-center gap-2.5">
            <span className="text-lg font-semibold">{planLabel(me.plan)}</span>
          </div>
          {/* What this plan costs, in the currency this console is showing. It is the
              catalogue price, not a receipt: an account can also be put on a plan by an
              operator grant or a redemption code, and those leave no order to quote. */}
          {planPrice ? (
            <p className="mb-1.5 text-2xl font-semibold tracking-tight tabular-nums">
              {price(planPrice, currency)}
              {planPrice.duration_days ? (
                <span className="ml-1.5 text-[13.5px] font-medium text-muted-foreground">
                  {fill(t.everyDays, { days: planPrice.duration_days })}
                </span>
              ) : null}
            </p>
          ) : null}
          <p className="mb-5 text-[13.5px] leading-relaxed text-muted-foreground">
            {active && me.plan_expires_at
              ? `${t.until} ${formatDate(me.plan_expires_at, lang)}`
              : t.freeFallback}
          </p>
          <Button asChild className="mt-auto w-fit">
            <a href="/billing">{t.navBilling}</a>
          </Button>
        </Card>
      </div>

      <div className="mt-4 grid grid-cols-1 gap-4 md:grid-cols-3">
        <Stat label={t.creditBalance} value={usd(me.credits_cents)} sub={t.creditBalanceSub} />
        <Stat
          label={t.dailyFree}
          value={`${Math.round((me.free_points ?? 0) * 100) / 100} / ${me.free_points_daily ?? 0}`}
          sub={t.dailyFreeSub}
        />
        <Stat
          label={t.thisWeek}
          value={
            me.quota_weekly_cap_cents > 0
              ? `${usd(me.quota_week_used_cents)} / ${usd(me.quota_weekly_cap_cents)}`
              : usd(me.quota_week_used_cents)
          }
          sub={me.quota_week_reset_at ? `${t.resets} ${formatDate(me.quota_week_reset_at, lang)}` : undefined}
        />
      </div>

      <h2 className="mb-3 mt-8 text-sm font-semibold">{t.recentActivity}</h2>
      <UsageTable rows={usage?.recent ?? []} lang={lang} limit={6} />
    </div>
  );
}
