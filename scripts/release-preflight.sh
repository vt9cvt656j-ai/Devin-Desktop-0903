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

echo "── 有没有残留的 dmg 卷 ──"
# tauri 打 dmg 时会挂载一个卷做窗口布局；这一步失败的话卷不会被卸掉，
# 而**下一次打包会被它挡住**——报的却是一句没有信息的
# 「error running bundle_dmg.sh」，让人以为是签名或权限问题。
# 今天连撞两次才找到，清掉卷就成。
stale=$(ls -d /Volumes/dmg.* 2>/dev/null || true)
if [ -n "$stale" ]; then
  echo "   清理残留卷：$stale"
  for v in $stale; do hdiutil detach "$v" -force >/dev/null 2>&1 || true; done
fi

echo "── 双仓一致性 ──"
cargo test --manifest-path server/Cargo.toml repo_sync -- --nocapture >/dev/null 2>&1 \
  || fail "外层仓的 ide/ 副本和内层仓 HEAD 不一致。流水线编的是外层那份，直接打包会发出旧代码。
   先对齐：cargo test --manifest-path server/Cargo.toml repo_sync  看它列出哪些文件差了。"

echo "── 内层新增的文件有没有漏进外层 ──"
# repo_sync 比的是两边**已提交**的内容，所以它抓不到「内层新建、外层压根没这个文件」
# 这种情况——而流水线编的正是外层那份，漏一个模块就是整个功能不存在。
# 实际漏过一次：macos_tree.rs（原生 AX 快照）。
missing=$(git -C ide ls-files | while IFS= read -r f; do
  git ls-files --error-unmatch "ide/$f" >/dev/null 2>&1 || echo "$f"
done)
[ -z "$missing" ] || fail "这些文件内层有、外层没有（流水线编的是外层，等于它们不存在）：
$missing
   补上：git add ide/<文件> && git commit"

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

echo "── Windows 那条分支能不能编过 ──"
# 在 mac 上 cargo check 只编 mac 那些 cfg 分支，Windows 独家的代码根本不参与编译——
# 也就是说「本机全绿」对 Windows 版一点保证都没有。实际发生过：改了一个函数签名，
# mac 全过，Windows 侧四个 cfg 分支全部类型不匹配。
if command -v cargo-xwin >/dev/null 2>&1; then
  (cd ide/src-tauri && cargo xwin check --target x86_64-pc-windows-msvc >/dev/null 2>&1) \
    || fail "Windows 目标编不过：cd ide/src-tauri && cargo xwin check --target x86_64-pc-windows-msvc"
  # check **不做链接**，而有一整类失败只在链接期出现。实际发生过：build.rs 用
  # /MANIFESTINPUT 自己塞了一份 Windows 清单，而 tauri-build 本来就嵌一份——
  # 两份 MANIFEST 都是 id 1，链接直接失败（CVT1100 duplicate resource / LNK1123）。
  # 那次「补 DPI 感知」的修复因此让 Windows 整个编不出来，而这道闸门当时是绿的，
  # CI 上跑到链接那一步才炸。所以必须真链一次主程序。
  (cd ide/src-tauri && cargo xwin build --release --target x86_64-pc-windows-msvc --bin michael-ide >/dev/null 2>&1) \
    || fail "Windows 主程序链接不过（check 查不出来，只有真链接才会暴露）：cd ide/src-tauri && cargo xwin build --release --target x86_64-pc-windows-msvc --bin michael-ide"
  (cd ide/automation-framework && cargo xwin check --target x86_64-pc-windows-msvc --all-features >/dev/null 2>&1) \
    || fail "sidecar 的 Windows 目标编不过"
else
  echo "   （跳过：没装 cargo-xwin。要装：cargo install cargo-xwin）"
fi

echo "── 工具目录两份是否同步 ──"
(cd ide && node build/sync-tools-json.mjs --check >/dev/null 2>&1) \
  || fail "main.js 和 server/prompts/tools.json 的工具描述漂了。同步脚本只对齐 schema、
   不覆盖描述，所以描述改动必须两边都写。跑 node build/sync-tools-json.mjs 看差在哪。"

echo "── Actions 还跑不跑得起来（计费）──"
# 前面所有检查查的都是「代码能不能打出包」。这一条查的是「流水线**允不允许**开始跑」，
# 它和代码质量完全无关，却能让一次打 tag 彻底白费。
#
# 私有仓的 macOS runner 按 10 倍计费，一次三平台打包约 700 计费分钟。额度烧穿之后，
# 三个 job **一个都不会启动**，整个 run 5–9 秒就 failure。最坑的是失败原因
# **只在 annotation 里**，`gh run view --log-failed` 什么都看不到（日志本来就不存在），
# 于是现象是「刚才还好好的，现在 7 秒就挂，没有任何日志」——极容易被当成代码问题去查。
# 2026-08-21 的 v0.4.10 就是这么废掉的：tag 推了、包没出、release 也没建。
#
# 计费额度的 REST 接口要 `user` scope，而发版用的 token 通常没有（gh 会报 404 +
# "needs the user scope"）。所以这里不查额度，直接查**最近有没有出现过这个形状的失败**：
# 极短耗时 + failure + annotation 里带计费字样。这一条不需要任何额外权限。
if command -v gh >/dev/null 2>&1; then
  billing_hit=""
  # 只看最近 5 次；再往前的失败可能早就被处理掉了。
  for rid in $(gh run list --limit 5 --json databaseId,conclusion \
                 --jq '.[] | select(.conclusion=="failure") | .databaseId' 2>/dev/null); do
    if gh run view "$rid" 2>/dev/null | grep -qiE "spending limit|payments have failed"; then
      billing_hit="$rid"
      break
    fi
  done
  if [ -n "$billing_hit" ]; then
    fail "GitHub Actions 现在跑不起来：最近一次失败（run $billing_hit）的原因是账户计费
   —— 'recent account payments have failed or your spending limit needs to be increased'。
   三个打包 job 一个都不会启动，现在打 tag 只会白费一个版本号。
   先去 GitHub → Settings → Billing & plans 处理付款/提高额度，再回来发版。
   注意：**别用「把仓库改成公开」来绕过**。这个仓库是刻意保持私有的——里面有未混淆的
   main.js，而整条打包流水线的意义就是剥掉它的工具描述和编排逻辑。
   验证是否解封（7 秒出结果、不产生费用）：
     gh workflow run main.yml --ref \$(git rev-parse --abbrev-ref HEAD) && sleep 12 && gh run list --limit 1"
  fi
else
  echo "   （跳过：没装 gh CLI）"
fi

echo
echo "✅ 全部通过，可以打 tag 了。"
