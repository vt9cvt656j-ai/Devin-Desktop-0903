#!/usr/bin/env bash
#
# 构建并发布 Web 版 IDE。
#
# 为什么要有这个脚本：此前 `dist-web/` 是**手工构建后提交进 git**、再手工 rsync 上线的。
# 结果是产物和源码各走各的——实测仓库里那份产物比 src/main.js 落后 11 天，而生产上跑的
# 正是它。没有任何环节会告诉你"你部署的不是你写的代码"。
#
# 这里把三件事绑在一起：**每次都从当前源码重新构建 → 校验产物 → 才允许上线**。
#
# 用法：
#   ./scripts/deploy-web.sh              # 构建 + 校验，不上传（默认，安全）
#   ./scripts/deploy-web.sh --publish    # 校验通过后 rsync 到生产
set -euo pipefail

cd "$(dirname "$0")/.."

SSH_KEY="${MICHAEL_SSH_KEY:-$HOME/.ssh/michael_server}"
REMOTE="${MICHAEL_WEB_REMOTE:-root@154.44.13.133}"
REMOTE_DIR="${MICHAEL_WEB_DIR:-/var/www/michael-ide-app/}"
PUBLISH=0
[ "${1:-}" = "--publish" ] && PUBLISH=1

echo "==> 安装依赖（npm ci，锁定 lockfile）"
npm ci

echo "==> 构建（--base=/app/：线上挂在 /app/ 子路径，base 不对整站资源 404）"
rm -rf dist
npx vite build --base=/app/

# ── 产物校验：这几条任何一条不过都不许上线 ────────────────────────────────────
echo "==> 校验产物"

main_js=$(ls dist/assets/main-*.js 2>/dev/null | head -1 || true)
if [ -z "$main_js" ]; then
  echo "✗ 没找到 dist/assets/main-*.js —— 构建没产出主 bundle" >&2
  exit 1
fi

# 1) 混淆确实生效。混淆是构建最后一步写回磁盘的，跑没跑过只能看产物。
if ! grep -q '_0x' "$main_js"; then
  echo "✗ $main_js 里没有混淆标记（_0x）——混淆没生效，别把明文源码传上去" >&2
  exit 1
fi

# 2) 没有源码副本混进产物。仓库里存在 main.js.bak / main.js.pre-*.bak 这类文件，
#    一旦被打包或 rsync 上去，混淆就完全白做了。
if find dist -name '*.bak' -o -name '*.map' | grep -q .; then
  echo "✗ 产物里含 .bak/.map 文件：" >&2
  find dist \( -name '*.bak' -o -name '*.map' \) >&2
  exit 1
fi

# 3) base 前缀正确，否则线上所有资源 404。
if ! grep -q '/app/assets/' dist/index.html; then
  echo "✗ dist/index.html 里没有 /app/assets/ 前缀——base 没设对" >&2
  exit 1
fi

echo "✓ 产物校验通过：$(basename "$main_js")，$(du -sh dist | cut -f1)"

if [ "$PUBLISH" -ne 1 ]; then
  echo
  echo "（未上传。确认无误后加 --publish 发布到 $REMOTE:$REMOTE_DIR）"
  exit 0
fi

echo "==> 发布到 $REMOTE:$REMOTE_DIR"
# --delete：线上要和本次构建完全一致，不留上一版的残余 chunk。
rsync -az --delete -e "ssh -i $SSH_KEY" dist/ "$REMOTE:$REMOTE_DIR"
echo "✓ 已发布。生产上的主 bundle："
ssh -i "$SSH_KEY" "$REMOTE" "ls -la ${REMOTE_DIR}assets/main-*.js"
