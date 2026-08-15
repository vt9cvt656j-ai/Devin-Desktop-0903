import type React from "react";
import { useEffect, useState } from "react";
import { Apple } from "lucide-react";
import { Button } from "@/components/ui/button";
import { mseFetch } from "@/lib/mse";
import { cn } from "@/lib/utils";

/*
 * 下载地址来自网关，不是 GitHub。
 *
 * 发行仓库是私有的，github.com/.../releases/latest 对访客一律 404 —— 这里以前直接写死
 * 那个链接，等于每个点"下载"的人都被送进一个 404。网关本来就为此准备了公开代下载路由
 * （update.rs：带 token 取私有仓库资产，并把清单里的地址改写成
 * /api/ide/update/download/<tag>/<file>），所以站点读同一份清单就够了。
 *
 * 没有已发布版本时清单返回 204。那时不摆死链接：明说桌面版还没放出来，把人引到注册。
 * 运营在控制台发布 release 之后，这里下一次加载自动变成真实下载，站点不用再改。
 */
const UPDATE_FEED = "https://code.mrday.one/api/ide/update";
/**
 * The installers that exist right now.
 *
 * The feed above only answers when a release carries `latest.json`, the signed manifest
 * the auto-updater needs — and the published release predates that file, so it answers
 * "no update" and this page concluded there was nothing to download at all. Installing and
 * auto-updating are different questions: this endpoint reads the release's own assets, so
 * the buttons offer what is actually published.
 */
const DOWNLOADS = "https://code.mrday.one/api/ide/downloads";
const SIGN_UP = "https://code.mrday.one/gate";

/** lucide has no Windows glyph; this is the four-pane mark. */
function WindowsMark({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" aria-hidden className={className} fill="currentColor">
      <path d="M3 5.4 10.3 4.4v7.1H3zM11.2 4.3 21 3v8.5h-9.8zM3 12.5h7.3v7.1L3 18.6zM11.2 12.5H21V21l-9.8-1.3z" />
    </svg>
  );
}

type OS = "mac" | "windows";

/** Tauri's updater target keys — how the manifest names each build. */
const TARGET: Record<OS, string> = { mac: "darwin-aarch64", windows: "windows-x86_64" };

type Release =
  | { state: "checking" }
  | { state: "ready"; version: string; urls: Partial<Record<OS, string>> }
  | { state: "none" };

/** Best guess at the visitor's platform; both downloads stay reachable either way. */
function useDetectedOS(): OS {
  const [os, setOs] = useState<OS>("mac");
  useEffect(() => {
    const ua =
      (navigator as { userAgentData?: { platform?: string } }).userAgentData?.platform ||
      navigator.userAgent ||
      "";
    if (/win/i.test(ua)) setOs("windows");
  }, []);
  return os;
}

/**
 * The published installers, or `none` if there genuinely are not any.
 *
 * Kept separate from the manifest path above so the two reasons a download can be missing
 * stay distinguishable: "no signed manifest" is common and recoverable, "no release at
 * all" is the only one that should send someone to a sign-up link.
 */
async function installersOrNone(): Promise<Release> {
  try {
    const res = await mseFetch(DOWNLOADS, { cache: "no-store" });
    if (!res.ok) return { state: "none" };
    const body = (await res.json()) as { version?: string; mac?: string | null; windows?: string | null };
    const urls: Partial<Record<OS, string>> = {};
    if (body.mac) urls.mac = body.mac;
    if (body.windows) urls.windows = body.windows;
    return Object.keys(urls).length
      ? { state: "ready", version: body.version ?? "", urls }
      : { state: "none" };
  } catch {
    return { state: "none" };
  }
}

function useRelease(): Release {
  const [release, setRelease] = useState<Release>({ state: "checking" });
  useEffect(() => {
    let alive = true;
    void (async () => {
      try {
        const res = await mseFetch(UPDATE_FEED, { cache: "no-store" });
        // 204 means the gateway is healthy and simply has no published release. Sealed or
        // not, this reads the handler's own status — a sealed 204 carries no body and is
        // rebuilt as a bodyless 204, so the distinction below survives encryption.
        if (res.status !== 200) throw new Error(String(res.status));
        const manifest = (await res.json()) as {
          version?: string;
          platforms?: Record<string, { url?: string }>;
        };
        const urls: Partial<Record<OS, string>> = {};
        for (const os of ["mac", "windows"] as OS[]) {
          const url = manifest.platforms?.[TARGET[os]]?.url;
          if (url) urls[os] = url;
        }
        if (!alive) return;
        if (Object.keys(urls).length) {
          setRelease({ state: "ready", version: manifest.version ?? "", urls });
          return;
        }
        throw new Error("manifest carried no platforms");
      } catch {
        // No manifest is not the same as nothing to install. Ask what is actually
        // published before giving up and showing a sign-up link.
        if (alive) setRelease(await installersOrNone());
      }
    })();
    return () => {
      alive = false;
    };
  }, []);
  return release;
}

type IconType = React.ComponentType<{ className?: string }>;

const PLATFORMS: Record<OS, { label: string; requirement: string; icon: IconType }> = {
  mac: {
    label: "Download for MacOS",
    // The published disk image is universal, so it covers both architectures. Saying
    // "Apple Silicon" here turned away Intel Mac owners the build actually supports.
    requirement: "MacOS 13+ · Intel & Apple Silicon · .dmg",
    icon: Apple,
  },
  windows: {
    label: "Download for Windows",
    requirement: "Windows 10+ · x64 · .exe or .msi",
    icon: WindowsMark,
  },
};

export function DownloadButtons({
  size = "lg",
  variant = "onLight",
  className,
}: {
  size?: "md" | "lg";
  /** The CTA band is dark, so the secondary button needs a different treatment there. */
  variant?: "onLight" | "onDark";
  className?: string;
}) {
  const detected = useDetectedOS();
  const release = useRelease();
  const other: OS = detected === "mac" ? "windows" : "mac";
  const Primary = PLATFORMS[detected].icon;
  const Secondary = PLATFORMS[other].icon;
  const muted = variant === "onDark" ? "text-primary-foreground/60" : "text-muted-foreground";

  // Nothing published: say so, and offer what does work today rather than a dead button.
  if (release.state === "none") {
    return (
      <div className={cn("flex flex-col items-center gap-3", className)}>
        <Button size={size} variant={variant === "onDark" ? "inverse" : "default"} asChild>
          <a href={SIGN_UP}>Create an account</a>
        </Button>
        <p className={cn("max-w-md text-center text-xs leading-relaxed", muted)}>
          The MacOS and Windows builds are not published yet. Create an account now — the editor
          runs in your browser, and the download appears here the moment it ships.
        </p>
      </div>
    );
  }

  const checking = release.state === "checking";
  const href = (os: OS) => (release.state === "ready" ? release.urls[os] : undefined);
  const live = (os: OS) => !checking && !!href(os);

  return (
    <div className={cn("flex flex-col items-center gap-3", className)}>
      <div className="flex flex-col items-center gap-3 sm:flex-row">
        <Button
          size={size}
          variant={variant === "onDark" ? "inverse" : "default"}
          disabled={!live(detected)}
          asChild={live(detected)}
        >
          {live(detected) ? (
            <a href={href(detected)}>
              <Primary /> {PLATFORMS[detected].label}
            </a>
          ) : (
            <span>
              <Primary /> {PLATFORMS[detected].label}
            </span>
          )}
        </Button>
        <Button
          size={size}
          variant="outline"
          disabled={!live(other)}
          asChild={live(other)}
          className={cn(
            variant === "onDark" &&
              "border-primary-foreground/25 bg-transparent text-primary-foreground hover:bg-primary-foreground/10",
          )}
        >
          {live(other) ? (
            <a href={href(other)}>
              <Secondary /> {PLATFORMS[other].label}
            </a>
          ) : (
            <span>
              <Secondary /> {PLATFORMS[other].label}
            </span>
          )}
        </Button>
      </div>

      <p className={cn("text-xs", muted)}>
        {release.state === "ready" && release.version ? (
          <>
            Version {release.version}
            <span className="mx-1.5 opacity-50">·</span>
          </>
        ) : null}
        {PLATFORMS[detected].requirement}
        <span className="mx-1.5 opacity-50">·</span>
        {PLATFORMS[other].requirement}
      </p>
    </div>
  );
}
