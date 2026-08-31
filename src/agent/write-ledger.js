/**
 * run._writeLedger 的读法。
 *
 * 账本是**顺序追加、整 run 不剪枝**的：一个文件写失败、下一轮重试成功，两条都留在
 * 账上。所以「哪些文件此刻没落盘」只能**按 path 取最后一条**，不能 filter(ok === false)
 * ——后者会把已经补救成功的也算进去。
 *
 * 这件事踩过一次：收尾门（run._incompleteReason）一直是对的，而喂给模型的
 * _deliveryFactsLine 用的是 filter，于是重试成功之后**每一轮**还在告诉模型
 * 「src/api/invoices.ts 此刻不在磁盘上，不要说它已保存」。模型要么把刚写对的文件
 * 从头再整篇写一遍覆盖掉，要么收尾对用户说「没能保存成功，请手动检查」，而文件
 * 好好躺在磁盘上。两个消费方读同一本账，判据只能有一个——就是这里。
 *
 * `a.ok === true` 是严格的：只有明确记成功才算落盘，undefined / 缺字段一律当没落盘。
 * 沿用收尾门原本的写法，别放宽——放宽的方向是「谎报已保存」。
 */
export function failedWritePaths(ledger) {
  const last = new Map();
  for (const a of (Array.isArray(ledger) ? ledger : [])) if (a?.path) last.set(String(a.path), a.ok === true);
  return [...last].filter(([, ok]) => !ok).map(([path]) => path).filter(Boolean);
}
