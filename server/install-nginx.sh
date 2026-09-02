#!/usr/bin/env bash
# 把仓库里的 nginx 配置装到系统里。由 deploy.sh 在**远端**、在 flock 内、以 $REMOTE_DIR
# 为工作目录调用，且必须排在 rollout.sh **之前**（配置与颜色无关，先装配置再切颜色）。
#
# ## 为什么需要它
#
# `/etc/nginx/` 下那几份和 `server/nginx/` 下的同名文件此前是**手工拷过去的副本**（inode
# 不同，不是链接），而 deploy.sh 完全不碰它们。后果有两个方向：
#   · 改了仓库不生效 —— 提交完以为上线了，其实线上还是旧的；
#   · 有人直接改线上不留痕 —— 仓库和实际长期不一致，而没有任何东西会发现。
# 这次给 michael-backend.conf 加 upstream 间接层时就是手工装的，装完才想起来这件事本身
# 就该自动化。
#
# ## 失败时的行为
#
# 校验不过就**整体还原并让部署失败**，绝不留下半装的配置。顺序也是刻意的：装配置在起容器
# 之前，所以配置写错时容器一个都没动过。
#
# ## 一个不装的文件
#
# `michael-backend-upstream.conf` 是 rollout.sh 的生成物，指向此刻在服务的那种颜色。
# 它在仓库里**没有副本**，这里也绝不写它 —— 按字面装一份写死蓝色端口的进去，而绿色正在
# 服务的话，那一下就是把 nginx 指向一个已经停掉的端口，全站 502。
set -euo pipefail

SRC="${SRC:-./nginx}"
BAKDIR="${BAKDIR:-/root/nginx-backups}"
TS="$(date +%Y%m%d-%H%M%S)"

say() { printf '  [nginx] %s\n' "$*"; }

# 仓库文件 → 系统落点。左边是 $SRC 下的文件名，右边是绝对路径。
PAIRS=(
  "michael-backend.conf|/etc/nginx/sites-available/michael-backend"
  "michael-limits.conf|/etc/nginx/conf.d/michael-limits.conf"
  "michael-security-headers.conf|/etc/nginx/snippets/michael-security-headers.conf"
  "michael-headers-account.conf|/etc/nginx/snippets/michael-headers-account.conf"
  "cloudflare-real-ip.conf|/etc/nginx/snippets/cloudflare-real-ip.conf"
  "michaelide-sites.conf|/etc/nginx/sites-available/michaelide-sites"
  "mrday-site.conf|/etc/nginx/sites-enabled/mrday-site"
)

mkdir -p "$BAKDIR"
changed=()
restore_list=()

for pair in "${PAIRS[@]}"; do
  name="${pair%%|*}"
  dest="${pair##*|}"
  src="$SRC/$name"

  if [ ! -f "$src" ]; then
    say "仓库里没有 $name —— 跳过（这条落点保持现状）"
    continue
  fi
  # 内容一样就什么都不做：绝大多数部署都会走到这里，不该产生备份、也不该 reload。
  if [ -f "$dest" ] && cmp -s "$src" "$dest"; then
    continue
  fi

  if [ -f "$dest" ]; then
    cp "$dest" "$BAKDIR/$(basename "$dest").$TS"
    restore_list+=("$BAKDIR/$(basename "$dest").$TS|$dest")
  else
    # 目标本来不存在：还原时要删掉而不是拷回去。
    restore_list+=("|$dest")
  fi
  install -m 0644 "$src" "$dest"
  changed+=("$name")
  say "已更新 $dest"
done

if [ "${#changed[@]}" -eq 0 ]; then
  say "配置与仓库一致，无需改动"
  exit 0
fi

say "校验（nginx -t）"
if nginx -t 2>&1 | sed 's/^/  [nginx] /'; then
  systemctl reload nginx
  say "已 reload（优雅，不断连接）；本次更新：${changed[*]}"
else
  say "校验不通过 —— 整体还原，并让这次部署失败（容器一个都还没动）"
  for entry in "${restore_list[@]}"; do
    bak="${entry%%|*}"
    dest="${entry##*|}"
    if [ -n "$bak" ]; then cp "$bak" "$dest"; else rm -f "$dest"; fi
  done
  nginx -t >/dev/null 2>&1 || say "警告：还原后仍不通过，需要人工介入"
  exit 1
fi
