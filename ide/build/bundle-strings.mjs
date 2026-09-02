// 从**混淆产物**里还原「拿到这个包的人实际读得到的字符串全集」。
//
// 为什么不能 grep 明文：javascript-obfuscator 的 stringArrayThreshold=0.75 把大约四分之三
// 的字面量搬进了 base64 编码的字符串表。实测本仓 dist/assets/main-*.js：
//     【何时用】   表里 95 条 / 明文 42 条
//     【vs 替代】  表里 68 条 / 明文  0 条   ← grep 报「干净」
//     example_call 形状  表里 99 条 / 明文  0 条   ← grep 报「干净」
// 也就是说以明文为判据的泄漏检查会对 69% 的泄漏面**结构性失明**，而且是安静地失明。
//
// 而且那张表用的是**打乱过的 base64 字母表**（本仓实测 'abcdefghijklmnopqrstuvwxyzABC…'，
// 小写在前），所以「把数组元素拿出来自己 atob」同样不成立。唯一站得住的做法是
// **把混淆器自己那两个函数（数组函数 + 解码函数）抠出来，在 vm 里真的跑一遍**。
//
// 混淆器换版本会不会失效：会——但只会**响亮地**失效。见 extractBundleStrings 末尾的
// 三条自检（认不出函数 / 解出来的条数对不上数组长度 / 阳性对照找不到），任何一条不满足
// 就 throw，不会退化成「扫不到＝没泄漏」。
import vm from "node:vm";

const IDENT = String.raw`(?:_0x[0-9a-fA-F]+|[A-Za-z_$][\w$]*)`;

// 从 `{` 起做配对扫描，跳过字符串/模板/正则里的花括号。
function matchBrace(src, open) {
  let depth = 0, quote = null;
  for (let i = open; i < src.length; i++) {
    const c = src[i];
    if (quote) {
      if (c === "\\") { i++; continue; }
      if (c === quote) quote = null;
      continue;
    }
    if (c === '"' || c === "'" || c === "`") { quote = c; continue; }
    if (c === "{") depth++;
    else if (c === "}" && --depth === 0) return i;
  }
  return -1;
}

function sliceFunction(src, startIdx) {
  const open = src.indexOf("{", startIdx);
  if (open < 0) return null;
  const close = matchBrace(src, open);
  return close < 0 ? null : src.slice(startIdx, close + 1);
}

/**
 * @param {string} src   一个混淆后的 chunk 的完整源码
 * @param {{canary?: string}} opts  阳性对照：一段**必然**在这个包里、且必然进了字符串表的文本。
 *                                  找不到它就说明抽取本身坏了，直接 throw。
 * @returns {{strings: string[], plaintext: string, diag: object}}
 */
export function extractBundleStrings(src, opts = {}) {
  const diag = { arrayFn: null, decoderFn: null, arrayLen: 0, base: null, decoded: 0, mode: null };

  // 数组函数： function A(){const B=['…','…',…];A=function(){return B;};return A();}
  const arrM = new RegExp(String.raw`function (${IDENT})\(\)\s*\{\s*const (${IDENT})\s*=\s*\[`).exec(src);
  // 解码函数： function D(i,k){i=i-<纯算术>; const a=A(); let s=a[i]; …}
  const decM = new RegExp(
    String.raw`function (${IDENT})\((${IDENT})\s*,\s*${IDENT}\)\s*\{\s*\2\s*=\s*\2\s*-\s*(\(?[^;]*?\)?);`,
  ).exec(src);

  if (!arrM || !decM) {
    throw new Error(
      "[bundle-strings] 认不出混淆器的字符串表结构（数组函数或解码函数没匹配上）。" +
      "多半是 javascript-obfuscator 换了版本或换了 stringArray 形状。" +
      "**不要**把这当成「没有泄漏」——先把这个抽取器修好，再谈扫描结果。",
    );
  }
  diag.arrayFn = arrM[1];
  diag.decoderFn = decM[1];

  const arrSrc = sliceFunction(src, arrM.index);
  const decSrc = sliceFunction(src, decM.index);
  if (!arrSrc || !decSrc) throw new Error("[bundle-strings] 函数体括号配对失败——抽取器坏了，不是包干净。");

  // 只跑这两个函数，**不跑**顶部那个洗牌 IIFE：洗牌只是把数组循环右移，
  // 我们要的是「字符串的集合」而不是「索引→字符串的映射」，集合不受洗牌影响。
  // 少跑一段就少一处会随混淆器改形状而崩的地方，而且避开了那个 while(!![]) 死循环风险。
  const ctx = vm.createContext(Object.create(null));
  vm.runInContext(
    `${arrSrc}\n${decSrc}\nglobalThis.__arr=${arrM[1]};globalThis.__dec=${decM[1]};`,
    ctx, { timeout: 15000 },
  );
  const rawLen = vm.runInContext("__arr().length", ctx, { timeout: 15000 });
  diag.arrayLen = rawLen;

  const out = new Set();
  const pull = (i) => {
    try {
      const s = vm.runInContext(`__dec(${i})`, ctx, { timeout: 2000 });
      if (typeof s === "string" && s.length) { out.add(s); return true; }
    } catch { /* 越界索引会抛，正常 */ }
    return false;
  };

  // 索引基偏移写在解码函数第一行的算术里（本仓实测 `i=i-(-0x5da+-0x85*-0x2b+-0x8*0x1ed)` → 277）。
  let base = null;
  try { base = vm.runInContext(`(${decM[3]})`, ctx, { timeout: 1000 }); } catch { /* 下面扫 */ }
  if (Number.isFinite(base)) {
    diag.base = base; diag.mode = "arith";
    for (let i = base; i < base + rawLen; i++) pull(i);
  }
  // 算不出（或算出来明显不对）就把索引空间扫一遍。慢，但绝不静默扫空。
  if (out.size < rawLen * 0.5) {
    diag.mode = diag.mode ? "arith+scan" : "scan";
    const hi = Math.max(rawLen * 4, 1 << 16);
    for (let i = 0; i < hi; i++) pull(i);
  }
  diag.decoded = out.size;

  // ——— 三条自检：抽取器坏了必须比「包不干净」更响 ———
  // 绝对下限只挡「压根没抓到表」。**不要**在这里写一个按大块估的数：
  // 同一次构建里 main-*.js 有 5.8 MB(16724 条) 和 82 KB(574 条) 两个块，
  // 一个绝对阈值必然误伤其中一个。按体积缩放的下限交给调用方（见 test/bundle-ip-leak）。
  if (rawLen < 32) {
    throw new Error(`[bundle-strings] 字符串表只有 ${rawLen} 条 —— 抽取器没抓到真正的表。`);
  }
  if (out.size < rawLen * 0.9) {
    throw new Error(`[bundle-strings] 只解出 ${out.size}/${rawLen} 条——解码函数的调用约定变了，扫描结果不可信。`);
  }
  if (opts.canary) {
    const hit = [...out].some((s) => s.includes(opts.canary)) || src.includes(opts.canary);
    if (!hit) {
      throw new Error(
        `[bundle-strings] 阳性对照 ${JSON.stringify(opts.canary)} 在解出的 ${out.size} 条字符串里找不到。` +
        "抽取器看不见它本该看见的东西，任何「没扫到泄漏」的结论都是假的。",
      );
    }
  }
  return { strings: [...out], plaintext: src, diag };
}

/** 表 + 明文，一起当作「读得到的文本」。未进表的那 25%（threshold）和 reservedStrings 都在明文里。 */
export function readableTextOf(src, opts) {
  const { strings, diag } = extractBundleStrings(src, opts);
  return { strings, plaintext: src, diag };
}
