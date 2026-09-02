import { useCallback, useEffect, useRef, useState } from "react";

export type FlashTone = "ok" | "error";

/**
 * 行级反馈：一行保存成功（或失败），就是这一行自己亮一下。
 *
 * 为什么不是 toast：操作员的眼睛在他刚点的那一行上，不在屏幕右上角。表格重拉之后所有行
 * 长得都一样，"刚才那笔到底确认上了没有"只能靠它自己说。240ms，颜色回到原样，见
 * index.css 的 [data-flash]。
 *
 * 同一时间只亮一行——两行同时亮，"哪一行是我刚动的"这个问题就又回来了。
 * 属性会在 duration 之后摘掉，所以同一行连点两次能重新播一遍动画，不需要换 key 重挂。
 */
export function useRowFlash(duration = 420) {
  const [flash, setFlash] = useState<{ id: string; tone: FlashTone } | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(
    () => () => {
      if (timer.current) clearTimeout(timer.current);
    },
    [],
  );

  const fire = useCallback(
    (id: string, tone: FlashTone = "ok") => {
      if (timer.current) clearTimeout(timer.current);
      setFlash({ id, tone });
      timer.current = setTimeout(() => setFlash(null), duration);
    },
    [duration],
  );

  /** 直接写成 <TableRow data-flash={toneOf(row.id)}>，没有匹配时是 undefined（不渲染属性）。 */
  const toneOf = useCallback(
    (id: string): FlashTone | undefined => (flash && flash.id === id ? flash.tone : undefined),
    [flash],
  );

  return { fire, toneOf };
}
