// vite 插件：把 src/tool-guides.js 的散文剥进发布构建。
//
// 落地方式（vite.config.js 里两处）：
//   import { stripToolGuidesPlugin } from "./build/strip-tool-guides-plugin.mjs";
//   plugins: [ stripToolIpPlugin(), stripToolGuidesPlugin(), react(), … ]
// 放在 stripToolIpPlugin() 旁边、react() 之前，两者都是 enforce:"pre"，顺序无关。
//
// 守卫的设计和 stripToolIpPlugin 同源（剥不到就让构建失败），但**多两条**，因为
// 「剥掉多少条」这个单一阈值抓不住本仓真实出现过的两种形状：
//   · 新加一张表（TOOL_EXAMPLES）→ 阈值照旧满足，新表原样进包  → 用 found/missing 抓
//   · 拼接串只剥了前半段（probe_env 等 4 条）→ 阈值照旧满足，后半句进包 → 用 residual 抓
import { stripToolGuides } from "./strip-tool-guides.mjs";

// 下限只回答一个问题：「这还是那个装着一百多条工具说明的文件吗」。
//
// 为什么是 400 而不是贴着当前值的 666：贴着当前值的阈值每加一个工具就要动一次，
// 于是它必然被人顺手调低，最后变成一个跟着现实走的数——那种阈值不守任何东西。
// 400 的含义是「掉了三分之一以上」：删掉整整一个大类（最大的 execution 类约 30 个工具
// × 4 个字段 = 120）仍然在 400 以上，而「作用域只认出了两张表里的一张」（最小的一张
// TOOL_EXAMPLES 是 98 条）会当场掉到 98，红。真要做那么大的结构调整，就该在这里
// 显式改一次阈值，而不是让它悄悄滑过去。
const MIN_CHANGED = 400;

export function stripToolGuidesPlugin() {
  return {
    name: "strip-tool-guides",
    apply: "build",
    enforce: "pre",
    transform(code, id) {
      if (!/\/src\/tool-guides\.js$/.test(id.replace(/\\/g, "/"))) return null;
      const r = stripToolGuides(code);

      if (!r.found) {
        throw new Error(
          `[strip-tool-guides] 在 src/tool-guides.js 里找不到这些表：${(r.missing || []).join(", ")}。`
          + "剥离没有执行，构建出来的包会带着 143 个工具的完整使用说明（【何时用】/【vs 替代】/"
          + "调用示例）。改了那个文件的结构就要同步 build/strip-tool-guides.mjs 的 STRIP_REGIONS。",
        );
      }
      // 「剥了一半」：这条抓的是拼接串 `usage_note: '前半'+'后半'` 只剥掉前半段。
      // 剥完之后首值确实是空串，任何看第一个字符的检查都会判它干净，而后半句进了包。
      if (r.residual !== 0) {
        throw new Error(
          `[strip-tool-guides] 剥完之后仍有 ${r.residual} 个非空的说明值留在表里 —— 这是一个剥了一半的包，拒绝发布。`,
        );
      }
      // 两个**互相独立**的计数器：一个逐字符走出来，一个朴素 split 数出来。
      // 只有一个计数器时，「实现和它自己的计数一起坏掉」是看不出来的。
      if (r.changed !== r.expected) {
        throw new Error(
          `[strip-tool-guides] 逐字符剥掉 ${r.changed} 个键，独立计数器数出应有 ${r.expected} 个 —— 有一批值没被剥到。`,
        );
      }
      if (r.changed < MIN_CHANGED) {
        throw new Error(
          `[strip-tool-guides] 只剥掉了 ${r.changed} 个值（当前实际应为 666）。这个文件的形状变了，`
          + "剥离作用域要重新核过，而不是把 MIN_CHANGED 调低。",
        );
      }
      return { code: r.code, map: null };
    },
  };
}
