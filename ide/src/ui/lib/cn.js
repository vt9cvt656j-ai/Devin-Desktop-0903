import { clsx } from "clsx";
import { twMerge } from "tailwind-merge";

/**
 * shadcn 的标准类名合并：clsx 处理条件类，twMerge 解决 Tailwind 类冲突
 * （后写的 `px-4` 能正确覆盖先写的 `px-2`，而不是两个都留在 class 里）。
 */
export function cn(...inputs) {
  return twMerge(clsx(inputs));
}
