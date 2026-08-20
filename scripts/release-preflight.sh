#!/usr/bin/env bash
# 发版前置检查。打 tag 之前跑这个，全绿再打。
#
# 为什么必须在本地跑，而不是做成 CI 的一步：
#   发版流水线住在外层仓，checkout 的也是外层仓，编的是外层仓里那份 ide/ 副本。
#   而 ide/ 同时是一个**独立的嵌套 git 仓**（不是 submodule），CI 里根本没有它的
#   历史可查。也就是说「外层那份副本比内层旧」这件事，CI 在结构上就检测不到——
#   它会安安静静地把一份旧代码打成包发出去。唯一能发现的地方就是这里。
set -euo pipefail
cd "$(dirname "$0")/.."
fail() { echo "❌ $*" >&2; exit 1; }

echo "── 双仓一致性 ──"
cargo test --manifest-path server/Cargo.toml repo_sync -- --nocapture >/dev/null 2>&1 \
  || fail "外层仓的 ide/ 副本和内层仓 HEAD 不一致。流水线编的是外层那份，直接打包会发出旧代码。
   先对齐：cargo test --manifest-path server/Cargo.toml repo_sync  看它列出哪些文件差了。"

echo "── sidecar 是否比源码新（四个平台）──"
newest_src=$(find ide/automation-framework/src -name '*.rs' -newer ide/src-tauri/binaries/automation-server-aarch64-apple-darwin -print -quit 2>/dev/null || true)
[ -z "$newest_src" ] || fail "automation-server (aarch64) 比它的源码旧：$newest_src
   automation-framework 是独立 crate，Tauri 不会替你重编。四个平台都要重编，否则
   通用包和 Windows 包会带着旧代码出去，而 build.rs 的守卫只检查当前构建目标那一个。"
for b in universal-apple-darwin x86_64-apple-darwin x86_64-pc-windows-msvc.exe; do
  f="ide/src-tauri/binaries/automation-server-$b"
  [ -e "$f" ] || fail "缺少 $f"
  stale=$(find ide/automation-framework/src -name '*.rs' -newer "$f" -print -quit 2>/dev/null || true)
  [ -z "$stale" ] || fail "$f 比源码旧（例如 $stale）"
done

echo "── 三套测试 ──"
(cd ide && npm test >/dev/null 2>&1) || fail "前端测试没过：cd ide && npm test"
cargo test --manifest-path ide/src-tauri/Cargo.toml --lib >/dev/null 2>&1 || fail "原生层测试没过"
cargo test --manifest-path server/Cargo.toml >/dev/null 2>&1 || fail "网关测试没过"
(cd ide/automation-framework && cargo test --all-features >/dev/null 2>&1) || fail "自动化框架测试没过"

echo "── 工具目录两份是否同步 ──"
(cd ide && node build/sync-tools-json.mjs --check >/dev/null 2>&1) \
  || fail "main.js 和 server/prompts/tools.json 的工具描述漂了。同步脚本只对齐 schema、
   不覆盖描述，所以描述改动必须两边都写。跑 node build/sync-tools-json.mjs 看差在哪。"

echo
echo "✅ 全部通过，可以打 tag 了。"
