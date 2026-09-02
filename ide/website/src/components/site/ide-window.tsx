import ideLight from "@/assets/ide-real-light.png";
import ideDark from "@/assets/ide-real-dark.png";
import { cn } from "@/lib/utils";

/*
 * 真实产品画面：不再手搭 DOM 仿制。
 * 两张图由无头浏览器对 ide 本体（npm run dev 预览）逐主题截取，@2x，
 * 内容即真实 IDE：Monaco、资源管理器、Assistant、状态栏，一个像素不差。
 */
export function IdeWindow({ className }: { className?: string }) {
  return (
    <div
      className={cn(
        "brand-ring overflow-hidden rounded-xl border border-border bg-card",
        className,
      )}
      style={{
        boxShadow:
          "var(--ide-shadow), 0 24px 80px -24px color-mix(in srgb, var(--brand) 25%, transparent)",
      }}
    >
      <img
        src={ideLight}
        width={1360}
        height={850}
        alt="Mr. Day One in light mode: the file explorer, a TypeScript file open in the Monaco editor, and the AI assistant panel"
        className="block w-full dark:hidden"
      />
      <img
        src={ideDark}
        width={1360}
        height={850}
        alt="Mr. Day One in dark mode: the file explorer, a TypeScript file open in the Monaco editor, and the AI assistant panel"
        className="hidden w-full dark:block"
      />
    </div>
  );
}
