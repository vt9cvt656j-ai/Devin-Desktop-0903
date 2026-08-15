#!/usr/bin/env bash
# 用固定的代码签名身份构建 Mr. Day One。
#
# 为什么需要这个脚本
# ------------------
# macOS 的隐私授权（辅助功能 / 屏幕录制 / 自动化）不是记在 bundle id 上的，而是记在
# **代码签名的指定要求**上。ad-hoc 签名（tauri.conf.json 里的 `signingIdentity: "-"`）
# 拿不出证书，系统只能退而用 cdhash 当要求：
#
#     designated => cdhash H"ffdbf5a0…"
#
# 这就把权限钉死在了"某一次构建"上。每发一版新的，cdhash 就变一次，之前的授权当场
# 作废——而系统设置里的开关**仍然显示为打开**（那个勾是 auth_value，不是校验结果）。
# 用户看到的就是"权限明明开着却说没权限"，而且每次更新都会重来一遍。
#
# 用证书签名之后，指定要求变成：
#
#     designated => identifier "ai.devin.ide" and anchor apple generic
#                   and certificate leaf[subject.OU] = <TEAM_ID>
#
# 它跨构建稳定，于是用户授一次权就一直有效。
#
# 用法
# ----
#   APPLE_SIGNING_IDENTITY="Developer ID Application: 你的名字 (TEAMID)" ./scripts/build-signed.sh
#
# 顺带公证（用户下载后不会被 Gatekeeper 拦），再多给三个：
#   APPLE_ID / APPLE_PASSWORD（App 专用密码）/ APPLE_TEAM_ID
#
# 查本机有哪些可用身份：
#   security find-identity -v -p codesigning
set -euo pipefail
cd "$(dirname "$0")/.."

if [ -z "${APPLE_SIGNING_IDENTITY:-}" ]; then
  cat >&2 <<'MSG'
未设置 APPLE_SIGNING_IDENTITY，拒绝构建。

直接跑 `npm run tauri build` 会用 ad-hoc 签名，那样每发一版用户的隐私授权都会失效，
而且系统设置里的开关还照样亮着——这正是要根治的问题，不应该被无声地绕过。

本机可用的签名身份：
MSG
  security find-identity -v -p codesigning >&2 || true
  cat >&2 <<'MSG'

一个都没有的话，先跑一次：

    ./scripts/setup-signing-cert.sh

它会造一张自签的代码签名证书。**对权限来说自签就够了**——签名要求嵌在二进制里，
换台机器一样成立，所有用户都受益（实测：同一张证书签两个内容不同的二进制，指定
要求逐字节相同）。它不解决的只有 Gatekeeper：别人下载后首次打开要右键「打开」。
要连这个也免掉，才需要 Apple 开发者证书（$99/年）做公证，那时把
APPLE_SIGNING_IDENTITY 换成 Developer ID 即可，本脚本不用改。
MSG
  exit 1
fi

echo "签名身份：$APPLE_SIGNING_IDENTITY"

# macOS 会给执行过的文件盖上 com.apple.provenance，它会让 cargo 的硬链接/克隆失败，
# 而 xattr -d 删不掉——只能重写文件换个 inode。
cleaned=0
for f in src-tauri/gen/schemas/* src-tauri/binaries/* target/release/build/*/build_script_build-*; do
  [ -f "$f" ] || continue
  if xattr -p com.apple.provenance "$f" >/dev/null 2>&1; then
    cp "$f" "$f.tmp" && mv -f "$f.tmp" "$f" && cleaned=$((cleaned + 1))
  fi
done
echo "预清 provenance：$cleaned 个"

# 只打 .app。DMG 那步会走 hdiutil internet-enable，在这台机器上稳定失败，而它跟
# 签名身份、跟能不能跑毫无关系——让它把整条构建拖挂是纯粹的干扰。要 DMG 时单独加。
# 更新签名的私钥必须在这里注入。
#
# 这个脚本原来只管 Apple 代码签名，完全没有 TAURI_SIGNING_PRIVATE_KEY——而带密钥的是
# package.json 里的 `build:signed`。于是走这个脚本构建时，Tauri 会打完包再抱怨
# 「A public key has been found, but no private key」并以非零退出：**.app 其实已经
# 造好了，失败的只是更新产物的签名**。这个失败模式很坏——退出码说失败、产物却在，
# 一眼看去像构建挂了；而真正的后果是 .app.tar.gz 没有有效 .sig，发上去客户端会拒绝安装。
UPDATER_KEY_FILE="${TAURI_SIGNING_KEY_FILE:-$HOME/.tauri/michael-ide.key}"
if [ -f "$UPDATER_KEY_FILE" ]; then
  export TAURI_SIGNING_PRIVATE_KEY="$(cat "$UPDATER_KEY_FILE")"
  export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="${TAURI_SIGNING_PRIVATE_KEY_PASSWORD:-}"
  echo "更新签名私钥：已注入（$UPDATER_KEY_FILE）"
else
  echo "更新签名私钥：$UPDATER_KEY_FILE 不存在 —— 本次产物无法用于自动更新" >&2
  echo "（本地自用可以忽略；要发版就必须有它，否则客户端会因签名验证失败拒绝安装）" >&2
fi

npm run tauri build -- \
  --bundles "${MRDAYONE_BUNDLES:-app}" \
  --config "{\"bundle\":{\"macOS\":{\"signingIdentity\":\"$APPLE_SIGNING_IDENTITY\"}}}"

# 签名产物要跟这次的包对得上。.sig 比 .app.tar.gz 旧，说明这次没签成、留下的是上一次的，
# 而那份签名配不上这次的内容——发出去客户端一律验签失败。宁可现在红，也别发出去才发现。
if [ -n "${TAURI_SIGNING_PRIVATE_KEY:-}" ]; then
  _bundle_dir="target/release/bundle/macos"
  for _tar in "$_bundle_dir"/*.app.tar.gz; do
    [ -f "$_tar" ] || continue
    if [ ! -f "$_tar.sig" ]; then
      echo "更新产物缺少签名：$_tar.sig 不存在" >&2; exit 1
    fi
    if [ "$_tar.sig" -ot "$_tar" ]; then
      echo "更新产物的签名比包还旧：$_tar.sig —— 这是上一次的签名，配不上这次的内容" >&2; exit 1
    fi
    echo "更新产物签名：$(basename "$_tar").sig ✓"
  done
fi

APP="target/release/bundle/macos/Mr. Day One.app"

# 验收：签出来的东西必须真的不再钉在 cdhash 上，否则这次构建等于白签。
echo
echo "── 校验签名身份是否稳定 ──"
req="$(codesign -d -r- "$APP" 2>&1 | sed -n 's/^#\{0,1\} *designated => //p')"
echo "指定要求：$req"
case "$req" in
  cdhash*)
    echo "❌ 仍然钉在 cdhash 上——权限还是会在下次构建时失效。检查证书是否真的被用上了。" >&2
    exit 1
    ;;
  "")
    echo "❌ 读不到指定要求，无法确认。" >&2
    exit 1
    ;;
  *)
    echo "✅ 身份跨构建稳定，用户授权一次即可长期有效。"
    ;;
esac
