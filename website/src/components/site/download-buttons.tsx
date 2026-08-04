import type React from "react";
import { useEffect, useState } from "react";
import { Apple } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

/*
 * 下载区：两个平台都真实存在 —— CI 的 ide-package 矩阵产出
 * macOS Apple Silicon DMG 与 Windows x64 EXE/MSI。文案只说构建出来的东西：
 * 没有 Intel Mac 构建，就不写 Intel。
 */
const RELEASES = "https://github.com/fendoushaonian/Devin-Desktop/releases/latest";

/** lucide has no Windows glyph; this is the four-pane mark. */
function WindowsMark({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" aria-hidden className={className} fill="currentColor">
      <path d="M3 5.4 10.3 4.4v7.1H3zM11.2 4.3 21 3v8.5h-9.8zM3 12.5h7.3v7.1L3 18.6zM11.2 12.5H21V21l-9.8-1.3z" />
    </svg>
  );
}

type OS = "mac" | "windows";

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

type IconType = React.ComponentType<{ className?: string }>;

const PLATFORMS: Record<OS, { label: string; requirement: string; icon: IconType }> = {
  mac: {
    label: "Download for macOS",
    requirement: "macOS 13+ · Apple Silicon · .dmg",
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
  const other: OS = detected === "mac" ? "windows" : "mac";
  const Primary = PLATFORMS[detected].icon;
  const Secondary = PLATFORMS[other].icon;

  return (
    <div className={cn("flex flex-col items-center gap-3", className)}>
      <div className="flex flex-col items-center gap-3 sm:flex-row">
        <Button size={size} variant={variant === "onDark" ? "inverse" : "default"} asChild>
          <a href={RELEASES} target="_blank" rel="noreferrer">
            <Primary /> {PLATFORMS[detected].label}
          </a>
        </Button>
        <Button
          size={size}
          variant="outline"
          className={cn(
            variant === "onDark" &&
              "border-primary-foreground/25 bg-transparent text-primary-foreground hover:bg-primary-foreground/10",
          )}
          asChild
        >
          <a href={RELEASES} target="_blank" rel="noreferrer">
            <Secondary /> {PLATFORMS[other].label}
          </a>
        </Button>
      </div>

      <p
        className={cn(
          "text-xs",
          variant === "onDark" ? "text-primary-foreground/60" : "text-muted-foreground",
        )}
      >
        {PLATFORMS[detected].requirement}
        <span className="mx-1.5 opacity-50">·</span>
        {PLATFORMS[other].requirement}
      </p>
    </div>
  );
}
