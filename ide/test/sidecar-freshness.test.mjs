// 手工提交的 sidecar 二进制不许比它的源码旧。
//
// `automation-server` 是**独立 crate**：Tauri 不会自动重编它（没有依赖边），所以改了
// automation-framework 的源码、只重编 src-tauri，跑起来的还是旧二进制——改了等于没改，
// 而且不报错。四个平台的产物都是手工编、手工提交的。
//
// ── 为什么还要这条，src-tauri/build.rs 里明明已经有一道闸 ──────────────────────
//
// 那道闸判的是**文件修改时间**（源码比二进制新就 panic）。它在开发机上有效，
// 但在**干净检出上恒绿**：git 按路径字节序写盘，`automation-framework/` 排在
// `src-tauri/` 前面，于是 exe 的 mtime 反而更新——CI 每次都放行，安静地把旧二进制打进包。
//
// 2026-08-26 实测就是这个形状：Windows 的 exe 停在 08-23 的提交，而 rpc.rs / system.rs
// 已经到 08-25（那次新增了 screen.displays、修了 keyboard.type 的换行）。也就是说
// 从 HEAD 打的 Windows 包，自动化 sidecar 缺多显示器枚举、换行仍然静默丢失。
//
// 这条改判**提交时间**（git log），CI 查得到，mtime 那条查不到。两条并存不是重复：
// 一条守本机改完忘了重编，一条守检出之后的漂移。
import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { existsSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "..");

/** 某个路径最后一次提交的 UNIX 时间戳。不在 git 仓库里 / 没有提交时返回 null。 */
function lastCommitTs(relPath) {
  try {
    const out = execFileSync("git", ["log", "-1", "--format=%ct", "--", relPath], {
      cwd: ROOT, encoding: "utf8", stdio: ["ignore", "pipe", "ignore"],
    }).trim();
    return out ? Number(out) : null;
  } catch {
    return null;
  }
}

test("每个 sidecar 二进制都不比 automation-framework 的源码旧", () => {
  const binDir = join(ROOT, "src-tauri/binaries");
  if (!existsSync(binDir)) return; // 没有产物目录就没什么可守的

  const srcTs = lastCommitTs("automation-framework/src");
  if (srcTs === null) {
    // 不在 git 仓库里（比如从压缩包解出来跑测试）——这条判据没有输入，跳过是诚实的。
    return;
  }

  const stale = [];
  for (const name of readdirSync(binDir)) {
    if (!name.startsWith("automation-server-")) continue;
    const ts = lastCommitTs(`src-tauri/binaries/${name}`);
    if (ts === null) continue; // 还没提交过（本机刚编出来）——那不是"旧"
    if (ts < srcTs) {
      const d = (x) => new Date(x * 1000).toISOString().slice(0, 10);
      stale.push(`${name}（${d(ts)}） < 源码（${d(srcTs)}）`);
    }
  }

  assert.deepEqual(stale, [],
    "这些 sidecar 二进制比 automation-framework 的源码旧，打进包就是「你以为修好了」的版本：\n  "
    + stale.join("\n  ")
    + "\n重编命令（Windows 那份要交叉编译，本机装了 cargo-xwin 就能跑）：\n"
    + "  cd automation-framework && cargo build --release --target <三元组>\n"
    + "  cd automation-framework && cargo xwin build --release --target x86_64-pc-windows-msvc\n"
    + "然后把产物拷进 src-tauri/binaries/automation-server-<三元组>[.exe] 一起提交。");
});

test("build.rs 那道 mtime 闸还在（两条守的不是同一件事）", () => {
  // 这条守「别把那道闸当成重复劳动删掉」：mtime 那条抓的是本机改完忘了重编，
  // 上面这条抓的是检出之后的漂移。删掉任何一条都会漏掉一整类。
  const src = readdirSync(join(ROOT, "src-tauri")).includes("build.rs")
    ? execFileSync("cat", [join(ROOT, "src-tauri/build.rs")], { encoding: "utf8" })
    : "";
  assert.match(src, /newest_mtime/,
    "build.rs 的 mtime 闸没了——那条抓的是本机改完忘重编，git 这条抓不到");
  assert.match(src, /automation-framework\/src/,
    "build.rs 不再盯着 automation-framework 的源码了");
});
