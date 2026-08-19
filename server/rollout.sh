#!/usr/bin/env bash
# 蓝绿切换。由 deploy.sh 在**远端**、在 flock 内、以 $REMOTE_DIR 为工作目录调用。
#
# ## 为什么要有它
#
# 原来的发布是 `docker compose up -d --build`，也就是**先毁后建**：在确认新镜像跑得
# 起来之前，正在服务的容器已经没了。线上量到的代价有两笔：
#
#   · `upstream prematurely closed connection` —— 旧容器带着在途请求被停掉。
#     2026-08-19 09:27 那一次掐断的正是运营自己的一轮对话。
#   · `connect() failed (111)` —— 新容器起来之前的 1–2 秒，nginx 连不上后端，硬 502。
#
# 两周共 1,499 次，其中 276 次是 /v1/chat/completions。**全部落在部署时刻，日常运行
# 零次**——这台机器峰值 45 次/分钟、容量余量约 180 倍，从来不是并发问题。
#
# 更要命的是没有回滚：新版起不来时旧的已经被销毁了，只能干等健康检查超时。
#
# ## 顺序就是全部
#
#   1. 构建镜像                 失败 → 什么都没动
#   2. 起另一种颜色（新端口）    失败 → 什么都没动，旧的还在服务
#   3. 对新端口做健康检查        失败 → 拆掉新的，退出，旧的还在服务
#   4. 改 upstream 一行 + nginx -t   失败 → 还原并重测，旧的还在服务
#   5. nginx reload             这一刻起新流量走新颜色；旧连接在旧 worker 上继续
#   6. 等旧端口排空（有上限）
#   7. 拆掉旧颜色
#
# 第 5 步之前的任何失败都不影响正在服务的容器。这比原来的顺序**更**安全，不只是更快。
#
# ## 一个必须知道的取舍
#
# 第 5、6 步之间新旧两版**同时**连着同一个数据库（最长到 DRAIN 上限）。所以数据库迁移
# 必须对旧版保持兼容：加列、加表可以；删列、改类型、加不带默认值的 NOT NULL 会在这几十秒
# 里把还在服务的旧版打挂。原来的先毁后建没有重叠窗口，是拿停机换来的——现在换成了这条约束。
set -euo pipefail

: "${COMPOSE_PROJECT:?rollout.sh 需要 COMPOSE_PROJECT}"
: "${ENV_FILE:?rollout.sh 需要 ENV_FILE}"
: "${COMPOSE_FILES:?rollout.sh 需要 COMPOSE_FILES}"

# 端口。BACKEND_PORT 写在服务器的 .env 里（compose 靠 --env-file 读它），但那个文件
# **不会**自动进到这个脚本的环境里——所以按需从同一个文件取，两边看到的是同一个数。
env_val() { # $1=键名
  [ -f "$ENV_FILE" ] || return 0
  sed -nE "s/^[[:space:]]*$1[[:space:]]*=[[:space:]]*//p" "$ENV_FILE" | tail -1 | tr -d "\"'"
}
BLUE_PORT="${BACKEND_PORT:-$(env_val BACKEND_PORT)}"
BLUE_PORT="${BLUE_PORT:-8080}"
# 绿色端口刻意避开 8081：那是测试环境后端的端口，撞上的话一次生产切换就会抢掉它。
GREEN_PORT="${BACKEND_GREEN_PORT:-$(env_val BACKEND_GREEN_PORT)}"
GREEN_PORT="${GREEN_PORT:-8090}"
UPSTREAM_CONF="${UPSTREAM_CONF:-/etc/nginx/conf.d/michael-backend-upstream.conf}"
# 健康检查：每 2 秒一次。新版要跑迁移、预热模型目录、建索引，20 秒的 start_period 之后
# 还要留出余量，60 秒是实测冷启动（约 14 秒到 listening）的四倍。
HEALTH_TRIES="${HEALTH_TRIES:-30}"
# 排空上限。流式对话可以跑好几分钟，不可能等到零；到点就拆。即便如此也严格优于原来的
# 行为——原来是**立刻**把所有在途请求砍掉。
DRAIN_TRIES="${DRAIN_TRIES:-60}"

DC=(docker compose -p "$COMPOSE_PROJECT" --env-file "$ENV_FILE")
# COMPOSE_FILES 是 "-f a.yml -f b.yml" 这种形式，必须按词拆开。
# shellcheck disable=SC2206
DC+=( ${COMPOSE_FILES} )
# 绿色挂在 profile 后面。启动它和**拆掉**它都需要带上，否则 compose 当它不存在。
DC+=(--profile green)

say() { printf '  [rollout] %s\n' "$*"; }

# 现在 nginx 把流量指向哪个端口。文件不存在（第一次切换）→ 空，调用方按蓝色处理。
live_port() {
  [ -f "$UPSTREAM_CONF" ] || return 0
  sed -nE 's/^[[:space:]]*server[[:space:]]+127\.0\.0\.1:([0-9]+).*/\1/p' "$UPSTREAM_CONF" | head -1
}

wait_health() { # $1=端口
  local i
  for ((i = 0; i < HEALTH_TRIES; i++)); do
    if curl -fsS "http://127.0.0.1:$1/health" >/dev/null 2>&1; then
      say "新容器在 :$1 上健康（第 $((i + 1)) 次探测）"
      return 0
    fi
    sleep 2
  done
  return 1
}

established_on() { # $1=端口 —— 这个端口上还有多少条已建立的连接
  ss -tn state established "( sport = :$1 )" 2>/dev/null \
    | grep -c "127\.0\.0\.1:$1" || true
}

point_upstream_at() { # $1=端口
  local rollback
  rollback="$(mktemp)"
  if [ -f "$UPSTREAM_CONF" ]; then
    cp "$UPSTREAM_CONF" "$rollback"
  else
    : >"$rollback"
  fi
  # 先把整段文本算好再落盘：中途出错也不会留下一个被截断的配置文件。
  local body
  body="$(
    cat <<EOF
# 由 rollout.sh 生成——手工改会在下一次切换时被覆盖。当前指向端口 $1。
upstream michael_backend {
    server 127.0.0.1:$1;
}
EOF
  )"
  printf '%s\n' "$body" >"$UPSTREAM_CONF"
  if ! nginx -t 2>&1 | sed 's/^/  [nginx] /'; then
    say "nginx -t 失败，已还原 upstream 配置；旧容器仍在服务"
    if [ -s "$rollback" ]; then cp "$rollback" "$UPSTREAM_CONF"; else rm -f "$UPSTREAM_CONF"; fi
    nginx -t >/dev/null 2>&1 || say "警告：还原后 nginx -t 仍不通过，需要人工介入"
    rm -f "$rollback"
    return 1
  fi
  rm -f "$rollback"
  systemctl reload nginx
  say "nginx 已切到 :$1（reload 是优雅的，旧连接在旧 worker 上继续）"
}

drain() { # $1=端口
  local i n
  for ((i = 0; i < DRAIN_TRIES; i++)); do
    n="$(established_on "$1")"
    if [ "${n:-0}" -eq 0 ]; then
      say "旧端口 :$1 已排空（等了 $((i * 2)) 秒）"
      return 0
    fi
    sleep 2
  done
  say "旧端口 :$1 上仍有 ${n:-?} 条连接，达到 $((DRAIN_TRIES * 2)) 秒上限，继续拆除"
}

# ---------------------------------------------------------------- 主流程

"${DC[@]}" config --quiet

say "构建镜像（此时还不动任何容器）"
"${DC[@]}" build backend

CUR_PORT="$(live_port)"
CUR_PORT="${CUR_PORT:-$BLUE_PORT}"
if [ "$CUR_PORT" = "$BLUE_PORT" ]; then
  NEW_SVC=backend-green; NEW_PORT="$GREEN_PORT"; OLD_SVC=backend
else
  NEW_SVC=backend;       NEW_PORT="$BLUE_PORT";  OLD_SVC=backend-green
fi
say "当前在 :$CUR_PORT（$OLD_SVC）→ 目标 :$NEW_PORT（$NEW_SVC）"

# 上一次切换可能留下同名的残骸（比如中途失败）。先清掉，否则 up 会复用旧容器。
"${DC[@]}" rm -sf "$NEW_SVC" >/dev/null 2>&1 || true

say "启动 $NEW_SVC"
"${DC[@]}" up -d --no-deps "$NEW_SVC"

if ! wait_health "$NEW_PORT"; then
  say "新容器没能在 $((HEALTH_TRIES * 2)) 秒内变健康——拆掉它，保持旧版服务"
  "${DC[@]}" logs --tail=100 "$NEW_SVC" || true
  "${DC[@]}" rm -sf "$NEW_SVC" || true
  exit 1
fi

point_upstream_at "$NEW_PORT"

drain "$CUR_PORT"

# **停掉**旧颜色，不删。
#
# 一开始这里是 `rm -sf`，代价当场就付了：切换后想查十分钟前那几条 `upstream prematurely
# closed` 是怎么来的，旧容器连同它的日志已经不存在了——而部署前后正是最需要对照日志的时候。
# stop 之后容器保留（端口会释放，`restart: unless-stopped` 也不会把它拉回来），日志一直在，
# 直到下一次切换开头那句 `rm -sf "$NEW_SVC"` 把它清掉——也就是**总能回看上一版**。
say "停用 $OLD_SVC（保留容器与日志，供切换后对照；下一次切换时才清理）"
"${DC[@]}" stop "$OLD_SVC" || true

"${DC[@]}" ps
say "切换完成：流量在 :$NEW_PORT（$NEW_SVC）"
