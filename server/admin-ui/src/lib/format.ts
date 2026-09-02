/** Money and time formatting, in one place. The old file re-derived these inline ~20 times. */
export const cents = (v: number | null | undefined) =>
  v == null ? "—" : `$${(v / 100).toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;

export const num = (v: number | null | undefined) =>
  v == null ? "—" : v.toLocaleString("en-US");

export const when = (iso: string | null | undefined) => {
  if (!iso) return "—";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "—";
  const days = Math.floor((Date.now() - d.getTime()) / 86_400_000);
  if (days === 0) return "今天";
  if (days === 1) return "昨天";
  if (days < 30) return `${days} 天前`;
  return d.toLocaleDateString("zh-CN");
};
