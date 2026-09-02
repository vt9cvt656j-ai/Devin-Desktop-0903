#!/usr/bin/env bash
# 一次性：给 Mr. Day One 造一张固定的代码签名身份。
#
# 解决什么问题
# ------------
# macOS 把隐私授权（辅助功能 / 屏幕录制 / 自动化 / 完全磁盘访问）钉在**代码签名的
# 指定要求**上。ad-hoc 签名拿不出证书，系统只能退而用 cdhash 当要求：
#
#     designated => cdhash H"ffdbf5a0…"
#
# 于是权限被钉死在"某一次构建"上：每编译一版，cdhash 就变，之前授的权当场作废——
# 而系统设置里的开关**仍然显示为打开**（那个勾只是一条记录，不是校验结果）。
#
# 用一张证书签名之后，指定要求变成：
#
#     designated => identifier "ai.devin.ide" and certificate leaf = H"<证书哈希>"
#
# 它只跟证书有关，跟这一次编译出来的字节无关。实测：同一张证书签两个内容不同的
# 二进制，指定要求逐字节相同。所以授权一次就一直有效，以后每次更新都不会再掉。
#
# 自签证书够不够？
# ----------------
# 对**权限**来说够：签名要求嵌在二进制里，换台机器也一样成立，所有用户都受益。
# 它不解决的是 Gatekeeper——别人下载后首次打开仍要右键「打开」，或者你自己
# `xattr -dr com.apple.quarantine`。要连这个也免掉，才需要 Apple 开发者证书
# （$99/年）做公证；那时把 APPLE_SIGNING_IDENTITY 换成 Developer ID 即可，
# 本脚本和 build-signed.sh 都不用改。
#
# 用法：./scripts/setup-signing-cert.sh
# 撤销：security delete-certificate -c "Mr Day One Local Signing"
set -euo pipefail

CN="Mr Day One Local Signing"
WORK="$HOME/.mrday-signing"

if security find-certificate -c "$CN" >/dev/null 2>&1; then
  echo "✅ 证书「$CN」已存在，无需重复创建。"
  echo
  echo "构建命令："
  echo "  APPLE_SIGNING_IDENTITY=\"$CN\" ./scripts/build-signed.sh"
  exit 0
fi

mkdir -p "$WORK"
chmod 700 "$WORK"
cd "$WORK"

echo "── 1/3 生成自签代码签名证书 ──"
# extendedKeyUsage=codeSigning 是关键：没有它 codesign 不认这张证书。
openssl req -x509 -newkey rsa:2048 -keyout key.pem -out cert.pem -days 3650 -nodes \
  -subj "/CN=$CN" \
  -addext "basicConstraints=critical,CA:false" \
  -addext "keyUsage=critical,digitalSignature" \
  -addext "extendedKeyUsage=critical,codeSigning" 2>/dev/null
chmod 600 key.pem

# macOS 的 Security 框架不认 OpenSSL 3 默认的 PKCS12 MAC 算法，必须退回旧算法，
# 否则导入时报 "MAC verification failed"，看起来像密码错了其实不是。
openssl pkcs12 -export -out id.p12 -inkey key.pem -in cert.pem -passout pass:mrdayone \
  -name "$CN" -macalg sha1 -certpbe PBE-SHA1-3DES -keypbe PBE-SHA1-3DES 2>/dev/null

echo "── 2/3 导入登录钥匙串 ──"
security import id.p12 -k "$HOME/Library/Keychains/login.keychain-db" \
  -P mrdayone -T /usr/bin/codesign -A
rm -f id.p12   # 私钥已经在钥匙串里了，磁盘上不留第二份

echo "── 3/3 验证 codesign 能用它，且签出来的身份跨构建稳定 ──"
printf 'int main(){return 0;}\n' > a.c && cc -o a a.c
printf 'int main(){return 1;}\n' > b.c && cc -o b b.c
# 首次签名时 macOS 可能弹出钥匙串访问框，点「始终允许」即可（那是系统在问你，
# 是否允许 codesign 使用刚导入的私钥）。
codesign --force -s "$CN" --identifier ai.devin.ide.probe a
codesign --force -s "$CN" --identifier ai.devin.ide.probe b
ra="$(codesign -d -r- a 2>&1 | sed -n 's/^#\{0,1\} *designated => //p')"
rb="$(codesign -d -r- b 2>&1 | sed -n 's/^#\{0,1\} *designated => //p')"
rm -f a b a.c b.c

echo "  内容不同的两个二进制："
echo "    A: $ra"
echo "    B: $rb"
if [ -z "$ra" ] || [ "$ra" != "$rb" ]; then
  echo "❌ 指定要求不一致，说明还是钉在具体构建上——不要继续，先查证书。" >&2
  exit 1
fi
case "$ra" in
  cdhash*) echo "❌ 仍是 cdhash 形式，证书没被用上。" >&2; exit 1 ;;
esac

cat <<MSG

✅ 完成。这张证书给出的签名身份跨构建稳定，权限授一次就长期有效。

以后这样构建：
  APPLE_SIGNING_IDENTITY="$CN" ./scripts/build-signed.sh

装上新版之后还要**最后再授一次权**（这是从 ad-hoc 切到证书身份的那一次，之后不用了）：
  系统设置 → 隐私与安全性 → 辅助功能 → 选中 Mr. Day One 点「−」移除 → 点「+」
  重新添加 /Applications/Mr. Day One.app → 完全退出 App 再重新打开
  （屏幕录制、自动化 两处同理）

不想要了就撤销：security delete-certificate -c "$CN"
MSG
