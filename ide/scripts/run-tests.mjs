// 跨平台地跑 test/*.test.mjs。
//
// 为什么需要这个脚本，而不是直接 `node --test test/*.test.mjs`：
//
// 那条命令在 mac/Linux 上能跑，**靠的是 shell**：bash 在把参数交给 node 之前就已经
// 把通配符展开成 106 个文件名了。node 只看到一串具体路径，它认不认通配符根本不重要。
//
// Windows 上两件事同时不成立：
//   · GitHub Actions 的 `run:` 在 windows runner 上默认外壳是 **pwsh**，
//     而 pwsh 给**原生程序**传参时**不做通配符展开** —— node 收到的是字面量
//     `test/*.test.mjs`；改用 `npm test` 也一样，Windows 上 npm 用 cmd.exe 跑脚本，
//     cmd 同样不展开。
//   · 仓库钉的是 **node 20**，而 `--test` 自带的 glob 展开是 **node 21** 才加的。
// 两件事叠起来的结果是 `Could not find '…/test/*.test.mjs'`、exit 1 ——
// **一条测试都没跑**。（本机用 `npx node@20.19.0 --test 'test/*.test.mjs'` 复现过。）
//
// 这比「某几条红」危险得多：配上 continue-on-error 之后它显示成「带警告的通过」，
// 于是所有人以为 Windows 上跑过了 2749 条，实际是 0 条。
//
// **也不要改成 `node --test test/`**：node 会把那个目录下**所有** .js/.mjs 当测试文件，
// 包括 `test/helpers/source.mjs` 和 `test/e2e/run*.mjs` —— 后者会 `spawn("/bin/sh", …)`
// 并真的去 import 执行器跑起来。它们今天不在 `test/*.test.mjs` 的匹配范围内，
// 换成目录参数就会被拖进来。
//
// 所以：在 JS 里自己列文件（readdirSync，不依赖 node 20 没有的 globSync），
// 显式传给 `node --test`。两个平台走同一条路径。
import { readdirSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const TEST_DIR = join(ROOT, "test");

// 只收 test/ **直接子级**里以 .test.mjs 结尾的，和原来那个通配符逐字等价。
// 排序是为了让两次运行的顺序一致（readdirSync 在 NTFS 上有序、APFS 上无序）。
const files = readdirSync(TEST_DIR, { withFileTypes: true })
  .filter((e) => e.isFile() && e.name.endsWith(".test.mjs"))
  .map((e) => join("test", e.name))
  .sort();

if (files.length === 0) {
  console.error("test/ 下一个 *.test.mjs 都没有 —— 这不是「全部通过」，是没找到测试文件。");
  process.exit(1);
}

// 下限哨兵：真跑起来之前先确认收集到的数量是合理的。
// 少了一大截通常意味着上面的判据被改坏了，而那种情况下「全绿」是假的。
if (files.length < 50) {
  console.error(`只收集到 ${files.length} 个测试文件，明显偏少 —— 收集判据可能被改坏了。`);
  process.exit(1);
}

console.log(`跑 ${files.length} 个测试文件…`);
const r = spawnSync(process.execPath, ["--test", ...files], { cwd: ROOT, stdio: "inherit" });
process.exit(r.status === null ? 1 : r.status);
