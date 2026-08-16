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
# 查本机有哪些签名身份（**不要加 -v**，理由见下面 resolve 那段）：
#   security find-identity -p codesigning
set -euo pipefail
cd "$(dirname "$0")/.."

# 身份从哪来：环境变量优先，其次读 tauri.conf.json 里已经配好的那个。
#
# 原来这里是「没设 APPLE_SIGNING_IDENTITY 就拒绝构建」，理由写着「直接跑 tauri build
# 会用 ad-hoc 签名（配置里是 signingIdentity: "-"）」。**那个前提已经过时**——配置里
# 现在写的是 "Mr Day One Local Signing"，普通构建早就是正经证书签名的了。于是这个脚本
# 会对着一个配置齐全、能签、也一直在签的项目说「拒绝构建」。
if [ -z "${APPLE_SIGNING_IDENTITY:-}" ]; then
  APPLE_SIGNING_IDENTITY="$(python3 -c 'import json,sys
try:
    d=json.load(open("src-tauri/tauri.conf.json"))
    v=d.get("bundle",{}).get("macOS",{}).get("signingIdentity") or ""
except Exception:
    v=""
print("" if v.strip() in ("","-") else v)' 2>/dev/null || true)"
  [ -n "$APPLE_SIGNING_IDENTITY" ] && echo "签名身份取自 tauri.conf.json：$APPLE_SIGNING_IDENTITY"
fi

if [ -z "${APPLE_SIGNING_IDENTITY:-}" ]; then
  cat >&2 <<'MSG'
没有可用的代码签名身份（环境变量没设，tauri.conf.json 里也是 ad-hoc "-"），拒绝构建。

ad-hoc 签名拿不出证书，系统只能用 cdhash 当指定要求——每发一版 cdhash 就变一次，
用户之前授的隐私权限当场作废，而系统设置里的开关还照样亮着。这正是要根治的问题。

本机现有的签名身份（**故意不加 -v**：项目用的是自签证书，它一定带
CSSMERR_TP_NOT_TRUSTED 而被 -v 过滤掉——那个标记只说明「验证这张证书的链不受信任」，
**不影响用它签名**，实测每次构建都签成功）：
MSG
  security find-identity -p codesigning >&2 || true
  cat >&2 <<'MSG'

一个都没有的话，跑一次（它是幂等的，证书已存在会直接退出，不会造出第二张）：

    ./scripts/setup-signing-cert.sh

自签对权限来说就够了——签名要求嵌在二进制里，换台机器一样成立。它不解决的只有
Gatekeeper：别人下载后首次打开要右键「打开」。要连这个也免掉，才需要 Apple 开发者
证书（$99/年）做公证，那时把 APPLE_SIGNING_IDENTITY 换成 Developer ID 即可。
MSG
  exit 1
fi

# 身份得真的能用来签，而不只是名字对得上。私钥不在（只导入了证书没导入私钥）时，
# codesign 会在构建**最后一步**才失败——那时几分钟的编译已经白烧了。这里先花 0.1 秒问一句。
if ! security find-identity -p codesigning 2>/dev/null | grep -qF "$APPLE_SIGNING_IDENTITY"; then
  echo "钥匙串里找不到签名身份「$APPLE_SIGNING_IDENTITY」——先确认证书和私钥都在 login 钥匙串里。" >&2
  security find-identity -p codesigning >&2 || true
  exit 1
fi

echo "签名身份：$APPLE_SIGNING_IDENTITY"

# 目标架构。留空＝本机（Apple Silicon 上就是 aarch64）。
#
#   MRDAYONE_TARGET=x86_64-apple-darwin     ← Intel Mac 专用包
#   MRDAYONE_TARGET=universal-apple-darwin  ← 一个包通吃两种架构
#
# 指定 target 之后 cargo/Tauri 的产物会落到 target/<triple>/release/ 而不是 target/release/，
# 所以下面每一处路径都得跟着走。原来是三处硬编码 target/release/…：不改的话指定架构构建会
# 「成功但什么也找不到」——脚本对着本机架构的旧产物做校验，甚至给出通过的结论。
TARGET="${MRDAYONE_TARGET:-}"
TARGET_DIR="target${TARGET:+/$TARGET}/release"
[ -n "$TARGET" ] && echo "目标架构：$TARGET（产物落在 $TARGET_DIR/bundle/）" || echo "目标架构：本机"

# 先扫掉上一次没收干净的 DMG 中间产物。
#
# Tauri 的 bundle_dmg.sh 每次都建一个 `rw.<pid>.<产品名>.dmg`、挂载、填充、卸载、转换。
# 任何一次中断（构建被杀、卸载失败、机器睡了）都会让那个映像**永远挂着**，文件也留在
# target/ 下。攒够十来个之后 hdiutil attach 就会失败，而 Tauri 报出来的只有一句
# `failed to run bundle_dmg.sh`——既不说是挂载失败，也不说为什么。实测就是这么挂的：
# 8 个昨天留下的 rw 映像一直挂着，今天的构建直接打不出 DMG，而脚本还退出 0。
#
# 这一步只动 target/ 下**本产品自己**的临时读写映像，不碰任何别的磁盘映像。
_stale_mounts=0
while read -r _dev; do
  [ -n "$_dev" ] || continue
  hdiutil detach "$_dev" -force >/dev/null 2>&1 && _stale_mounts=$((_stale_mounts + 1))
done <<EOF
$(hdiutil info 2>/dev/null | awk -v pat="target/release/bundle/macos/rw\\." '
    $0 ~ /image-path/ && $0 ~ pat { p = 1 }
    p && /^\/dev\/disk[0-9]+\t/ { print $1; p = 0 }')
EOF
_stale_files=$(ls target/release/bundle/macos/rw.*.dmg 2>/dev/null | wc -l | tr -d " ")
rm -f target/release/bundle/macos/rw.*.dmg 2>/dev/null || true
if [ "$_stale_mounts" -gt 0 ] || [ "${_stale_files:-0}" -gt 0 ]; then
  echo "预清 DMG 残留：卸载 $_stale_mounts 个挂载、删除 ${_stale_files:-0} 个临时映像"
fi

# macOS 会给执行过的文件盖上 com.apple.provenance，它会让 cargo 的硬链接/克隆失败，
# 而 xattr -d 删不掉——只能重写文件换个 inode。
cleaned=0
for f in src-tauri/gen/schemas/* src-tauri/binaries/* "$TARGET_DIR"/build/*/build_script_build-*; do
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

# 记下开工时间，下面用它判断更新产物是不是**这一轮**造的。
#
# 清理统一走一个函数：后面校验那段还要挂载 DMG，而 `trap ... EXIT` 是**覆盖**不是追加——
# 各装各的必然只剩最后一个，前面那个临时文件就永远留在 /tmp 了。
_started="$(mktemp)"
_mounted=""
_cleanup() {
  rm -f "$_started" 2>/dev/null || true
  [ -n "$_mounted" ] && hdiutil detach "$_mounted" -quiet >/dev/null 2>&1
  _mounted=""
}
trap _cleanup EXIT

npm run tauri build -- \
  --bundles "${MRDAYONE_BUNDLES:-app}" \
  ${TARGET:+--target "$TARGET"} \
  --config "{\"bundle\":{\"macOS\":{\"signingIdentity\":\"$APPLE_SIGNING_IDENTITY\"}}}"

# 签名产物要跟这次的包对得上。.sig 比 .app.tar.gz 旧，说明这次没签成、留下的是上一次的，
# 而那份签名配不上这次的内容——发出去客户端一律验签失败。宁可现在红，也别发出去才发现。
if [ -n "${TAURI_SIGNING_PRIVATE_KEY:-}" ]; then
  _bundle_dir="$TARGET_DIR/bundle/macos"
  # 只校验**这次**的产物。
  #
  # 原来是遍历 *.app.tar.gz —— 于是应用改过名之后，目录里遗留的旧名产物
  # （Michael IDE.app.tar.gz，几个月前的）会让这道校验永远红：它的 .sig 当然比它自己
  # 还旧，但那跟这次构建毫无关系。校验该只认当前 productName 那一个。
  _product="$(python3 -c 'import json
try: print(json.load(open("src-tauri/tauri.conf.json")).get("productName") or "")
except Exception: print("")' 2>/dev/null || true)"
  _tar="$_bundle_dir/$_product.app.tar.gz"
  if [ -z "$_product" ] || [ ! -f "$_tar" ]; then
    echo "没有找到本次的更新产物（$_tar）——无法确认签名状态" >&2; exit 1
  fi
  # 它得是**这一轮**造出来的。
  #
  # 只打 dmg 时 Tauri 根本不产出 .app.tar.gz（它自己会警告 "no updater-enabled targets
  # were built"），而上一轮 app 构建留下的那份还躺在原地——.sig 和 .tar.gz 时间戳都是上次的、
  # 谁也不比谁旧，于是这道校验照样打勾说「更新产物签名 ✓」。人看到绿的就发出去了，实际发的
  # 是上一次的更新包。对一个靠自动更新推测试版的项目，这是最贵的一种假绿。
  if [ ! "$_tar" -nt "$_started" ]; then
    echo "更新产物不是这一轮造的：$_tar 比本次构建还旧。" >&2
    echo "（bundles=${MRDAYONE_BUNDLES:-app} 里没有 app 时不会产出 .app.tar.gz；要更新包就用 MRDAYONE_BUNDLES=app,dmg）" >&2
    exit 1
  fi
  if [ ! -f "$_tar.sig" ]; then
    echo "更新产物缺少签名：$_tar.sig 不存在" >&2; exit 1
  fi
  if [ "$_tar.sig" -ot "$_tar" ]; then
    echo "更新产物的签名比包还旧：$_tar.sig —— 这是上一次的签名，配不上这次的内容" >&2; exit 1
  fi
  echo "更新产物签名：$(basename "$_tar").sig ✓"
  # 旧名遗留物只提醒，不拦——它不影响这次发版，但留着会让人下次又困惑一遍。
  for _old in "$_bundle_dir"/*.app.tar.gz; do
    [ -f "$_old" ] || continue
    [ "$_old" = "$_tar" ] && continue
    echo "提示：目录里还有改名前的遗留产物 $(basename "$_old")（不影响本次，可以删）"
  done
fi

APP="$TARGET_DIR/bundle/macos/Mr. Day One.app"

# 验收对象要按**这次真的产出了什么**来找，不能把路径钉死。
#
# 打 DMG 时 Tauri 会在装完盘之后把 macos/ 下的 .app 清掉（日志里那句 "Cleaning …"）。
# 于是钉死路径的写法在 `MRDAYONE_BUNDLES=dmg` 这条路上，codesign 读到的永远是空，
# 脚本 exit 1，而 DMG 好好地躺在 bundle/dmg/ 里——退出码说失败、产物却在。这正是这个
# 脚本别处一直在骂的那种失败模式，只是发生在它自己身上。
# 而且后果比"看着吓人"严重：**真正发给用户的那条路，签名稳定性校验从来没跑过**。
#
# .app 还在就验它；被清掉了就挂载 DMG 验里面那份——那才是用户实际拿到的东西。
if [ ! -d "$APP" ]; then
  _dmg="$(ls -t "$TARGET_DIR"/bundle/dmg/*.dmg 2>/dev/null | head -1)"
  if [ -n "$_dmg" ]; then
    _mounted="$(mktemp -d)"
    if hdiutil attach "$_dmg" -nobrowse -readonly -mountpoint "$_mounted" -quiet; then
      APP="$_mounted/$(basename "$APP")"
      echo "（macos/ 下的 .app 已被 DMG 打包步骤清理，改为校验 $(basename "$_dmg") 里的那份）"
    else
      echo "挂载 $_dmg 失败，无法校验签名。" >&2; exit 1
    fi
  fi
fi

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
