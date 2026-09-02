#!/usr/bin/env node
/**
 * 把 MSE 客户端从唯一的源分发到三个前端。
 *
 * 唯一的源是 `server/web-shared/mse.ts`。官网、用户后台、管理后台各自 `src/lib/mse.ts`
 * 都是它的副本。
 *
 * 为什么是复制而不是共享一个包：官网在 `ide/` 里，那是一个**独立的 git 仓库**（见
 * repo_sync.rs 顶部），跨仓库的相对 import 会在构建时炸掉；给三个 Vite 应用各配一套
 * alias + tsconfig path 又要在三处维护同一件事。复制加一个守门测试更简单，也和这个
 * 仓库已有的做法一致。
 *
 * 副本是**生成物**，不要手改 —— `test/mse-sync.test.mjs` 会因为它和源不一致而失败。
 * 三份手抄的密码学代码里有一份改漏了，症状是那一个前端偶尔解不开，是最难查的一类故障。
 *
 *   node server/scripts/sync-mse-client.mjs          # 写入副本
 *   node server/scripts/sync-mse-client.mjs --check  # 只检查，不一致就非零退出
 */

import { readFileSync, writeFileSync, existsSync, mkdirSync, unlinkSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const serverRoot = resolve(here, "..");
const repoRoot = resolve(serverRoot, "..");

const SOURCE = resolve(serverRoot, "web-shared/mse.ts");

/** 三个前端。相对仓库根。 */
export const TARGETS = [
  "server/account-ui/src/lib/mse.ts",
  "server/admin-ui/src/lib/mse.ts",
  "ide/website/src/lib/mse.ts",
];

const BANNER = `/* eslint-disable */
// ⚠️ 生成文件，不要手改。
//
// 源：server/web-shared/mse.ts
// 重新生成：node server/scripts/sync-mse-client.mjs
//
// 手改这里的后果：test/mse-sync.test.mjs 会红，而且在它红之前，这一个前端和另外两个
// 已经在跑不同版本的密码学代码了。
`;

export function rendered() {
  return BANNER + "\n" + readFileSync(SOURCE, "utf8");
}

/*
 * 两个登录页拿不到 Vite。
 *
 * `ide/gate/gate.html` 和 `server/console-login/login.js` 是手写的原生页面，靠 scp
 * 部署，没有任何构建步骤 —— 但它们承载的是密码和验证码，是整个系统里最该加密的载荷。
 * 手抄一份原生 JS 版客户端等于埋一颗漂移的雷（见文件顶部），所以从同一份 TS 源编出一个
 * 自包含的 IIFE，挂成全局 `MSE`。
 *
 * esbuild 从三个前端的 node_modules 里借，不新增依赖。
 */
const BROWSER_BUNDLE = resolve(serverRoot, "web-shared/mse.browser.js");

const GATE_HTML = resolve(repoRoot, "ide/gate/gate.html");
const CONSOLE_MSE = resolve(serverRoot, "console-login/mse.js");
const GATE_BEGIN = "/* MSE-BUNDLE-BEGIN — 生成物，改 server/web-shared/mse.ts */";
const GATE_END = "/* MSE-BUNDLE-END */";

function esbuildBin() {
  for (const rel of [
    "account-ui/node_modules/.bin/esbuild",
    "admin-ui/node_modules/.bin/esbuild",
    "../ide/website/node_modules/.bin/esbuild",
  ]) {
    const p = resolve(serverRoot, rel);
    if (existsSync(p)) return p;
  }
  return null;
}

export function buildBrowserBundle() {
  const bin = esbuildBin();
  if (!bin) return null;
  // import.meta 在 IIFE 里不合法。两个登录页从不调用 mseEnvConfig()（它们自己写死
  // configureMse），所以把它替换成空对象即可，那条分支本来就有 try/catch 兜底。
  //
  // `--minify`：之前漏了，登录页那个包一直是**没压缩**的 —— 变量名、结构全在，等于把
  // 客户端源码原样贴出去。压缩是零成本零运行时代价的第一道，先收掉。
  let out = execFileSync(
    bin,
    [
      SOURCE,
      "--bundle",
      "--format=iife",
      "--global-name=MSE",
      "--target=es2020",
      "--define:import.meta={}",
      "--legal-comments=none",
      "--minify",
    ],
    { encoding: "utf8", maxBuffer: 16 * 1024 * 1024 },
  );

  // 第二道：控制流混淆。把它抬到「不是随便读读就能顺下来」的程度。
  //
  // 只对**这个登录页小包**做，不碰三个 SPA：SPA 是 500KB 的 React，重混淆会让它膨胀
  // 三倍、线上报错无从查，而 SPA 里真正敏感的也就是同一份加密客户端，那部分在这里已经
  // 覆盖。这个包只有十几 KB，膨胀有限。
  //
  // 诚实地说：这抬高的是**成本**，不是不可能性。协议本身是公开的（docs/MSE.md），
  // 密钥固定钉的是公开值。混淆挡不住铁了心的人，只是让顺手看一眼的人看不下去。
  const obf = obfuscate(out);
  if (obf) {
    out = obf;
  } else {
    console.log("  note  javascript-obfuscator 不可用，登录页包仅压缩未混淆");
  }
  return `// ⚠️ 生成物，不要手改。源：server/web-shared/mse.ts\n${out}`;
}

/**
 * 过一遍 javascript-obfuscator。拉不到就返回 null，调用方退回「仅压缩」。
 *
 * 用 npx 按需取，不写进任何 package.json 的 dependencies —— 它只在**构建期**跑，
 * 绝不进任何前端的运行时依赖树。和 esbuild 缺失时一样，工具不在不该让整条同步失败。
 *
 * 设 `MSE_NO_OBFUSCATE=1` 可跳过（本地快速迭代时省掉这一步）。
 */
function obfuscate(code) {
  if (process.env.MSE_NO_OBFUSCATE === "1") return null;
  // 先看本地 node_modules 里有没有（装过就用装好的，免得每次 npx 联网）。
  let bin = null;
  for (const rel of [
    "account-ui/node_modules/.bin/javascript-obfuscator",
    "admin-ui/node_modules/.bin/javascript-obfuscator",
    "../ide/website/node_modules/.bin/javascript-obfuscator",
  ]) {
    const p = resolve(serverRoot, rel);
    if (existsSync(p)) {
      bin = p;
      break;
    }
  }

  // 写到临时文件再跑，避免超长命令行。输入输出都在 scratch 目录。
  const scratch = resolve(serverRoot, "web-shared/.mse-obf.tmp.js");
  const outFile = resolve(serverRoot, "web-shared/.mse-obf.out.js");
  const OPTS = [
    scratch,
    "--output",
    outFile,
    // 固定种子 = 可复现输出。少了它，混淆器每次都换一套随机变量名和字符串数组顺序，
    // 于是 `--check` 的逐字节比对永远 DRIFT，sync 守卫就废了。固定种子不削弱强度
    // —— 混淆的安全性不依赖种子保密，攻击者拿到的输出两种情况下都一样 —— 它只是让
    // 「同一份源 → 同一份产物」重新成立，守卫才能继续防手改和防漏混淆。
    "--seed",
    "424242",
    "--compact",
    "true",
    "--control-flow-flattening",
    "true",
    "--control-flow-flattening-threshold",
    "0.75",
    "--string-array",
    "true",
    "--string-array-encoding",
    "base64",
    "--string-array-threshold",
    "0.8",
    "--dead-code-injection",
    "true",
    "--dead-code-injection-threshold",
    "0.3",
    "--identifier-names-generator",
    "mangled",
    "--transform-object-keys",
    "true",
    // 刻意**不开** self-defending / debug-protection。
    //
    // 这是登录页 —— 全站最不能坏的一页。self-defending 在源码被格式化时会自毁，
    // debug-protection 在 DevTools 打开时会卡死；两者都为了恶心逆向者，但只要有一丝
    // 概率在某个真实浏览器环境里误触发，代价就是**真人登不进去**。控制流平坦化 +
    // 字符串数组编码已经达到「顺手看一眼看不下去」的目标，不值得拿登录可用性去换那
    // 最后一点强度。
  ];
  try {
    writeFileSync(scratch, code);
    if (bin) {
      execFileSync(bin, OPTS, { stdio: "ignore" });
    } else {
      // 没装就 npx 按需拉。--yes 免交互；拉不到会抛，落到下面的 catch。
      execFileSync("npx", ["--yes", "javascript-obfuscator", ...OPTS], {
        stdio: "ignore",
      });
    }
    const result = readFileSync(outFile, "utf8");
    return result && result.length > 0 ? result : null;
  } catch {
    return null;
  } finally {
    for (const f of [scratch, outFile]) {
      try {
        if (existsSync(f)) unlinkSync(f);
      } catch {
        /* 清理失败无所谓，下次覆盖 */
      }
    }
  }
}

/** 把 bundle 塞进 gate.html 的标记区间。CSP 允许内联脚本，所以不需要额外的静态路由。 */
function injectIntoGate(html, bundle) {
  const a = html.indexOf(GATE_BEGIN);
  const b = html.indexOf(GATE_END);
  if (a < 0 || b < 0) return null;
  return html.slice(0, a + GATE_BEGIN.length) + "\n" + bundle + "\n" + html.slice(b);
}

function main() {
  const check = process.argv.includes("--check");
  const want = rendered();
  let bad = 0;

  for (const rel of TARGETS) {
    const path = resolve(repoRoot, rel);
    const current = existsSync(path) ? readFileSync(path, "utf8") : null;
    if (current === want) {
      console.log(`  ok    ${rel}`);
      continue;
    }
    if (check) {
      console.error(`  DRIFT ${rel}${current === null ? " (缺失)" : ""}`);
      bad += 1;
      continue;
    }
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, want);
    console.log(`  write ${rel}`);
  }

  // 原生页面用的浏览器包。esbuild 不在就跳过 —— 三个 TS 副本才是主线，
  // 缺个打包器不该让整条同步失败。
  const bundle = buildBrowserBundle();
  if (bundle === null) {
    console.log("  skip  mse.browser.js（找不到 esbuild，先在任一前端 npm install）");
  } else {
    for (const [label, path, want] of [
      ["web-shared/mse.browser.js", BROWSER_BUNDLE, bundle],
      ["server/console-login/mse.js", CONSOLE_MSE, bundle],
    ]) {
      const cur = existsSync(path) ? readFileSync(path, "utf8") : null;
      if (cur === want) {
        console.log(`  ok    ${label}`);
      } else if (check) {
        console.error(`  DRIFT ${label}`);
        bad += 1;
      } else {
        writeFileSync(path, want);
        console.log(`  write ${label}`);
      }
    }

    const gate = existsSync(GATE_HTML) ? readFileSync(GATE_HTML, "utf8") : null;
    if (gate === null) {
      console.log("  skip  ide/gate/gate.html（不在这个检出里）");
    } else {
      const next = injectIntoGate(gate, bundle);
      if (next === null) {
        console.error(`  DRIFT ide/gate/gate.html 里找不到 MSE-BUNDLE 标记`);
        bad += 1;
      } else if (next === gate) {
        console.log("  ok    ide/gate/gate.html");
      } else if (check) {
        console.error("  DRIFT ide/gate/gate.html");
        bad += 1;
      } else {
        writeFileSync(GATE_HTML, next);
        console.log("  write ide/gate/gate.html");
      }
    }
  }

  if (bad > 0) {
    console.error(`\n${bad} 处和 web-shared/mse.ts 不一致。跑一下：`);
    console.error("  node server/scripts/sync-mse-client.mjs");
    process.exit(1);
  }
}

// 被测试 import 时不要执行 main。
if (process.argv[1] && resolve(process.argv[1]) === resolve(fileURLToPath(import.meta.url))) {
  main();
}
