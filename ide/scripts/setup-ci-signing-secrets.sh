#!/usr/bin/env bash
# 一次性：把本机那张固定的代码签名证书交给 GitHub Actions，让 CI 打出来的包和本地构建
# 用**同一个签名身份**。
#
# 为什么非做不可
# --------------
# macOS 把隐私授权（辅助功能 / 屏幕录制 / 自动化 / 完全磁盘访问）钉在代码签名的
# **指定要求**上。没有证书时只能 ad-hoc 签名，指定要求退化成：
#
#     designated => cdhash H"ffdbf5a0…"
#
# 那是**这一次构建的字节哈希**。于是每发一版，用户之前授的权全部作废，而系统设置里的
# 开关还照样亮着——用户看到的就是「权限明明开着却说没权限」，且每次更新重来一遍。
# 这正是 28428fe 那次提交花力气根治的问题（详见 scripts/setup-signing-cert.sh 的注释）。
#
# 本地已经用证书签了，但 GitHub runner 的钥匙串是空的。CI 若不导入同一张证书，发出去的
# 安装包就会把这个 bug 原样带回来——而且没人会发现，因为构建是绿的。
#
# 用证书签之后指定要求变成 `identifier "ai.devin.ide" and certificate leaf = H"…"`，
# 只跟证书有关，跟构建出来的字节无关。所以 CI 和本地必须是**同一张**证书。
#
# 自签就够——它不解决的只有 Gatekeeper（用户首次打开要右键「打开」）。将来买了 Apple
# 开发者证书，把同样三个 secret 换成 Developer ID 的即可，流水线一行都不用改。
#
# 用法：./scripts/setup-ci-signing-secrets.sh
#      ./scripts/setup-ci-signing-secrets.sh --dry-run   ← 只做校验，不写 secret
# 撤销：gh secret delete APPLE_CERTIFICATE 等三个
set -euo pipefail

DRY_RUN=0
[ "${1:-}" = "--dry-run" ] && DRY_RUN=1
cd "$(dirname "$0")/.."

CN="$(python3 -c 'import json
try: print((json.load(open("src-tauri/tauri.conf.json")).get("bundle",{}).get("macOS",{}) or {}).get("signingIdentity") or "")
except Exception: print("")')"
if [ -z "$CN" ] || [ "$CN" = "-" ]; then
  echo "tauri.conf.json 里没有配置 bundle.macOS.signingIdentity（或还是 ad-hoc \"-\"）。" >&2
  echo "先跑 ./scripts/setup-signing-cert.sh 造证书，并把身份写进配置。" >&2
  exit 1
fi
echo "签名身份：$CN"

if [ "$DRY_RUN" = 0 ]; then
  command -v gh >/dev/null || { echo "需要 GitHub CLI（gh）。" >&2; exit 1; }
  gh auth status >/dev/null 2>&1 || { echo "gh 未登录，先跑 gh auth login。" >&2; exit 1; }
fi

# 证书 + 私钥打成 p12。
#
# 优先用 setup-signing-cert.sh 留在磁盘上的 PEM：从钥匙串导出会弹 GUI 授权框，在
# 自动化流程里会静默卡死（脚本看起来「还在跑」，其实在等一个没人看见的对话框）。
WORK="$HOME/.mrday-signing"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
chmod 700 "$TMP"
P12="$TMP/ci.p12"
# 密码每次现生成，不写死也不落盘——它跟着 secret 一起走。
P12_PWD="$(openssl rand -hex 24)"

if [ -f "$WORK/key.pem" ] && [ -f "$WORK/cert.pem" ]; then
  echo "── 从 $WORK 的 PEM 打包 p12 ──"
  # macOS 的 Security 框架不认 OpenSSL 3 默认的 PKCS12 算法，必须退回旧算法，否则
  # runner 上 security import 会报 "MAC verification failed"，看着像密码错了其实不是。
  openssl pkcs12 -export -out "$P12" -inkey "$WORK/key.pem" -in "$WORK/cert.pem" \
    -passout "pass:$P12_PWD" -name "$CN" \
    -macalg sha1 -certpbe PBE-SHA1-3DES -keypbe PBE-SHA1-3DES 2>/dev/null
else
  echo "── $WORK 下没有 PEM，改从钥匙串导出（可能会弹授权框，点「允许」）──"
  security export -k "$HOME/Library/Keychains/login.keychain-db" \
    -t identities -f pkcs12 -P "$P12_PWD" -o "$P12"
fi
[ -s "$P12" ] || { echo "p12 生成失败。" >&2; exit 1; }

# 发之前先自证：这份 p12 真的能签，而且签出来的指定要求跟本地一致。
#
# 不做这一步的话，一份坏掉的 p12 会一路传到 CI，在**十几分钟编译之后**才在最后一步
# codesign 上炸掉——那时白烧的机器时间已经花掉了。这里花两秒钱先问清楚。
echo "── 校验这份 p12 ──"
VERIFY_KC="$TMP/verify.keychain"
VERIFY_PWD="$(openssl rand -hex 16)"
security create-keychain -p "$VERIFY_PWD" "$VERIFY_KC"
security unlock-keychain -p "$VERIFY_PWD" "$VERIFY_KC"
security import "$P12" -k "$VERIFY_KC" -P "$P12_PWD" -T /usr/bin/codesign -T /usr/bin/security >/dev/null
security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "$VERIFY_PWD" "$VERIFY_KC" >/dev/null 2>&1
if ! security find-identity -p codesigning "$VERIFY_KC" 2>/dev/null | grep -qF "$CN"; then
  security delete-keychain "$VERIFY_KC" 2>/dev/null || true
  echo "❌ p12 导进独立钥匙串后找不到「$CN」——私钥八成没打进去，不要上传。" >&2
  exit 1
fi
# 签两个内容不同的二进制，指定要求必须逐字节相同、且不是 cdhash 形式。
# 这两条正是「权限不会在下次更新时失效」的充要条件。
printf 'int main(){return 0;}\n' > "$TMP/a.c" && cc -o "$TMP/a" "$TMP/a.c"
printf 'int main(){return 1;}\n' > "$TMP/b.c" && cc -o "$TMP/b" "$TMP/b.c"
_sign_req() {
  codesign --force -s "$CN" --keychain "$VERIFY_KC" --identifier ai.devin.ide.probe "$1" 2>/dev/null
  codesign -d -r- "$1" 2>&1 | sed -n 's/^#\{0,1\} *designated => //p'
}
RA="$(_sign_req "$TMP/a")"; RB="$(_sign_req "$TMP/b")"
security delete-keychain "$VERIFY_KC" 2>/dev/null || true
echo "  A: $RA"
echo "  B: $RB"
if [ -z "$RA" ] || [ "$RA" != "$RB" ]; then
  echo "❌ 两次签名的指定要求不一致——还是钉在具体构建上，不要上传。" >&2; exit 1
fi
case "$RA" in
  cdhash*) echo "❌ 指定要求仍是 cdhash 形式，证书没被用上，不要上传。" >&2; exit 1 ;;
esac
echo "✅ 这份 p12 签出来的身份跨构建稳定。"

if [ "$DRY_RUN" = 1 ]; then
  echo
  echo "── dry-run：以下 secret **没有**写入 ──"
  echo "  APPLE_CERTIFICATE           $(base64 < "$P12" | tr -d '\n' | wc -c | tr -d ' ') 字节 base64"
  echo "  APPLE_CERTIFICATE_PASSWORD  已生成（本次随机，未落盘）"
  echo "  APPLE_SIGNING_IDENTITY      $CN"
  echo
  echo "去掉 --dry-run 再跑一次即可真正写入。"
  exit 0
fi

echo "── 写入仓库 secret ──"
base64 < "$P12" | tr -d '\n' | gh secret set APPLE_CERTIFICATE
printf '%s' "$P12_PWD" | gh secret set APPLE_CERTIFICATE_PASSWORD
printf '%s' "$CN"      | gh secret set APPLE_SIGNING_IDENTITY

cat <<MSG

✅ 完成。CI 现在会用和本机同一张证书签名，用户授权一次即可长期有效。

已写入三个 secret：
  APPLE_CERTIFICATE           证书 + 私钥（p12，base64）
  APPLE_CERTIFICATE_PASSWORD  上面那份 p12 的密码（本次随机生成）
  APPLE_SIGNING_IDENTITY      $CN

还想连 Gatekeeper 一起免掉（用户下载后不用右键「打开」），再加三个走公证：
  gh secret set APPLE_ID / APPLE_PASSWORD / APPLE_TEAM_ID
流水线已经把它们透传给 tauri-action，配了就自动公证，没配就跳过。
MSG
