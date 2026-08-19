#!/usr/bin/env bash
# Deploy the backend to a server over SSH. Credentials stay in the caller's SSH
# agent or key file; .env is never copied from the development machine.
#
#   SERVER_HOST=154.44.13.133 SERVER_KEY=~/.ssh/michael_server ./deploy.sh
#
set -euo pipefail

: "${SERVER_HOST:?set SERVER_HOST, e.g. SERVER_HOST=154.44.13.133}"
SERVER_PORT="${SERVER_PORT:-22}"
SERVER_USER="${SERVER_USER:-root}"
SERVER_KEY="${SERVER_KEY:-}"
# 部署目标：prod（默认）或 test。
#
#   ./deploy.sh          → 生产
#   TARGET=test ./deploy.sh   → 测试环境（另一套容器、另一块数据库盘、另一个端口）
#
# 两套环境**必须**在这三样上分开，少一样就会踩到生产：
#   · REMOTE_DIR   —— rsync 的落点。同一个目录的话，一次测试部署就把生产的代码覆盖了。
#   · COMPOSE_PROJECT —— 容器名/网络/**卷名**的前缀。它决定 pgdata 是两块盘还是一块。
#   · BACKEND_PORT / SITES_DIR —— 宿主端口与静态站目录（走 .env，见 docker-compose.yml）。
TARGET="${TARGET:-prod}"
case "$TARGET" in
  prod)
    REMOTE_DIR="${REMOTE_DIR:-/opt/michael-ide-deploy/server}"
    COMPOSE_PROJECT="server"
    COMPOSE_FILES="-f docker-compose.yml"
    ENV_FILE=".env"
    HEALTH_PORT="8080"
    ;;
  test)
    REMOTE_DIR="${REMOTE_DIR:-/opt/michael-ide-deploy/server-test}"
    COMPOSE_PROJECT="server-test"
    COMPOSE_FILES="-f docker-compose.yml -f docker-compose.test.yml"
    ENV_FILE=".env.test"
    HEALTH_PORT="8081"
    ;;
  *)
    echo "TARGET 只能是 prod 或 test（收到：$TARGET）" >&2
    exit 1
    ;;
esac
echo "部署目标：$TARGET（项目 $COMPOSE_PROJECT，端口 $HEALTH_PORT，目录 $REMOTE_DIR）"
REMOTE="${SERVER_USER}@${SERVER_HOST}"
REMOTE_Q="$(printf '%q' "$REMOTE_DIR")"
# 锁按目标分开：测试环境的一次构建不该把生产的发布挡在门外。
REMOTE_LOCK="${REMOTE_LOCK:-/var/lock/michael-ide-deploy-${TARGET}.lock}"
REMOTE_LOCK_Q="$(printf '%q' "$REMOTE_LOCK")"
DEPLOY_LOCK_TIMEOUT_SECS="${DEPLOY_LOCK_TIMEOUT_SECS:-900}"

SSH_ARGS=(-p "$SERVER_PORT" -o BatchMode=yes -o ConnectTimeout=10 -o ConnectionAttempts=3)
if [[ -n "$SERVER_KEY" ]]; then
  SSH_ARGS+=(-i "$SERVER_KEY")
fi

# `status=$?` 放在 `if ssh …; then return 0; fi` 之后取到的是 **if 语句** 的退出码——
# 条件失败且没有 else 分支时它是 0。于是三次全失败之后这个函数返回 0，deploy.sh 一路走完
# 并打印 "deployment healthy"，实际什么都没部署。
#
# 这不是假想：2026-08-13 一次部署里远程构建因编译错误失败了三次，脚本照样报健康，
# 线上还跑着几小时前的旧镜像。同一个 bug 在本文件下面的 rsync_run 里已经被修过一次
# （注释就在那儿），ssh_run 这半边当时漏了。修法一样：用 `|| status=$?` 拿真实退出码，
# 它在 set -e 下也是安全的。
ssh_run() {
  local attempt status
  for attempt in 1 2 3 4 5; do
    status=0
    ssh "${SSH_ARGS[@]}" "$REMOTE" "$@" || status=$?
    if [ "$status" -eq 0 ]; then
      return 0
    fi
    # 255 是 ssh **自己**失败（握手掉线/连不上/认证不过），不是远端命令的返回值。
    # 这台机器的握手会随机以 "banner exchange ... invalid format" 掉，所以传输失败
    # 多给几次；远端命令自己报的错重试没意义，早点把真实退出码交出去。
    if [ "$status" -ne 255 ]; then
      echo "  remote step failed (exit ${status}); retrying" >&2
      [ "$attempt" -ge 3 ] && break
    else
      echo "  SSH 握手失败（第 ${attempt} 次，exit 255），重试…" >&2
    fi
    sleep $((attempt * 2))
  done
  echo "remote step failed after ${attempt} attempts (last exit ${status})" >&2
  return "$status"
}

# 远端真值探测：把「SSH 没连上」和「远端命令回答『否』」彻底分开。
#
# ssh 用 **255** 表示它自己失败；任何其它退出码都是远端命令自己的返回值。原来 .env
# 那道检查用的是 ssh_run，两者一律当成"远端说否"——于是这台机器上偶发的
# "banner exchange ... invalid format" 被翻译成了「服务器上没有 .env」。那是一条自信、
# 具体、而且完全错误的结论：.env 好端端地在那儿，照着这条提示去做（复制 .env.example
# 覆盖上去）会把生产的密码和密钥直接抹掉。
#
# 2026-08-19 实拍：连掉三次 → 脚本报 "No .env exists on the server" → 部署中止。
# 传输故障绝不能变成关于远端状态的断言；连不上就说连不上，什么都不判断。
ssh_true() {
  local attempt status
  for attempt in 1 2 3 4 5 6; do
    status=0
    ssh "${SSH_ARGS[@]}" "$REMOTE" "$@" >/dev/null 2>&1 || status=$?
    if [ "$status" -ne 255 ]; then
      return "$status"   # 远端真的回答了：0 = 是，非 0 = 否
    fi
    echo "  SSH 握手失败（第 ${attempt} 次），重试…" >&2
    sleep $((attempt * 3))
  done
  echo "" >&2
  echo "SSH 连不上 ${REMOTE}:${SERVER_PORT}（连续 6 次握手失败）。" >&2
  echo "这不是远端的状态，是连接问题——本次没有做任何判断，也没有改动服务器上任何东西。" >&2
  exit 2
}

RSYNC_RSH="ssh -p $(printf '%q' "$SERVER_PORT") -o BatchMode=yes -o ConnectTimeout=10 -o ConnectionAttempts=3"
if [[ -n "$SERVER_KEY" ]]; then
  RSYNC_RSH+=" -i $(printf '%q' "$SERVER_KEY")"
fi

echo "ensuring ${REMOTE_DIR} on ${REMOTE}:${SERVER_PORT}"
ssh_run "mkdir -p $REMOTE_Q"

echo "creating a pre-deploy backup when the installed backup script is available"
ssh_run "if test -x $REMOTE_Q/backup.sh; then $REMOTE_Q/backup.sh; fi"

echo "syncing source (excluding build output, dev dependencies and .env)"
# node_modules: the two admin frontends carry ~280MB of dev dependencies between them.
#   Nothing on the server builds them — they are built locally and their `dist/` is
#   published to /var/www by deploy-account-ui.sh — so syncing them shipped a third of a
#   gigabyte of development tooling onto the production host on every deploy, into a
#   directory that is otherwise 8MB. The Dockerfile never reads them either.
# --filter 'protect backups': --delete-delay removes anything on the server that is not
#   in this source tree, and `backups/` exists ONLY on the server (prompt rollbacks put
#   there deliberately, before this script ever ran). Without this line a routine deploy
#   silently deletes the rollbacks — which are exactly what someone reaches for when a
#   deploy goes wrong.
# Retried, like ssh_run above. This host drops a large share of SSH handshakes
# ("banner exchange: ... invalid format"), and a single dropped handshake mid-sync
# aborted the whole deploy with `unexpected end of file` — after the pre-deploy backup
# had already run, so it looked like a real failure rather than a flaky connection.
# rsync resumes from where it got to, so a retry is cheap and idempotent.
# `status=$?` used to sit AFTER an `if rsync …; then return 0; fi`, where `$?` is the exit status
# of the *if statement* — which is 0 when the condition failed and there is no else branch. So
# every failed attempt logged "failed (exit 0)" and, worse, the function returned 0 after all five
# attempts: the deploy carried on, rebuilt the container from the OLD files, health-checked green,
# and printed "deployment healthy" having shipped nothing. Observed for real (2026-08-10): five
# consecutive "unexpected end of file" rsync failures reported as a successful deploy. Capture the
# real exit code with `|| status=$?`, which is also safe under `set -e`.
rsync_run() {
  local attempt status
  for attempt in 1 2 3 4 5; do
    status=0
    # 注意 `--exclude '.env*'` 而不是 `--exclude .env`：rsync 带 --delete-delay，
    # 只排除 `.env` 的话，服务器上的 `.env.test`（本地没有这个文件）会被当成"多余文件"
    # **删掉** —— 表现是第一次测试部署报「No .env.test exists」，而它其实是刚被这次
    # rsync 删的。（注释必须写在命令**外面**：写在续行反斜杠后面会把命令截断。）
    rsync -az --delete-delay -e "$RSYNC_RSH" \
      --exclude target --exclude '.env*' --exclude .git \
      --exclude .DS_Store --exclude node_modules \
      --exclude '*.tsbuildinfo' \
      --filter 'protect backups' \
      --exclude '*.bak' --exclude '*.bak.*' --exclude '*.bak-*' --exclude '*.pre-*' \
      ./ "$REMOTE:$REMOTE_DIR/" || status=$?
    if [ "$status" -eq 0 ]; then
      return 0
    fi
    echo "  sync attempt ${attempt} failed (exit ${status}); retrying"
    sleep $((attempt * 3))
  done
  echo "source sync failed after 5 attempts (last exit ${status}) — NOT deploying stale files" >&2
  return "$status"
}
rsync_run

echo "checking for ${REMOTE_DIR}/${ENV_FILE} on the server"
if ! ssh_true "test -f $REMOTE_Q/$(printf '%q' "$ENV_FILE")"; then
  echo "No ${ENV_FILE} exists on the server. Copy .env.example to ${ENV_FILE} and fill in"
  echo "   JWT_SECRET / POSTGRES_PASSWORD / QQ_SMTP_* before the first run."
  if [ "$TARGET" = "test" ]; then
    echo "   测试环境还必须设 BACKEND_PORT=8081 和 SITES_DIR=/var/www/michael-sites-test，"
    echo "   否则它会去抢生产的端口、往生产的静态站目录里写。"
  fi
  exit 1
fi

echo "validating, updating and health-checking containers (serialized)"
# Compose replaces the single host-published backend container in place. Two
# deploys started together can therefore stop a freshly started container and
# leave nginx with a much longer 502 window. Hold a host-side flock through the
# replacement and health check so only one rollout can touch the project at a
# time. The lock is operational coordination only; it does not change request
# handling or access policy.
# `--env-file` 是给 **${VAR} 插值**用的（POSTGRES_USER / BACKEND_PORT / SITES_DIR 这些
# 写在 compose 文件里的占位）。它和服务里的 `env_file:` 是两件事：后者只把变量注入容器，
# 不参与插值。生产靠目录里那个 `.env` 被 Compose 自动加载，测试目录里没有 `.env`，
# 必须显式指过去，否则所有 ${VAR} 都会插成空串（表现是一堆 "variable is not set"）。
DC="docker compose -p $COMPOSE_PROJECT --env-file $ENV_FILE $COMPOSE_FILES"
if [ "$TARGET" = "prod" ]; then
  # 生产走蓝绿：先起新颜色、验康健、再切 nginx 的 upstream、最后停旧的。
  # 完整理由和失败时的行为写在 rollout.sh 顶部。
  #
  # 换掉的是原来的 `up -d --build` —— 它**先毁后建**：在确认新镜像跑得起来之前就把正在
  # 服务的容器销毁了。线上两周 1,499 次 `connect() failed (111)` / `upstream prematurely
  # closed`，**每一次都落在部署时刻**，日常运行零次。而且新版起不来时没有回滚可言。
  # 先装 nginx 配置、再切颜色。两件事的顺序是刻意的：
  #   · 配置与颜色无关，先装完就不会和切换互相干扰；
  #   · 配置校验不过时**容器一个都还没动**，这次部署干干净净地失败。
  # 在这之前 /etc/nginx 下的那几份是手工拷过去的副本，deploy.sh 完全不碰 —— 于是
  # 「改了仓库不生效」和「改了线上不留痕」两个方向的漂移都没人发现。
  REMOTE_DEPLOY_CMD="cd $REMOTE_Q && bash ./install-nginx.sh && COMPOSE_PROJECT='$COMPOSE_PROJECT' ENV_FILE='$ENV_FILE' COMPOSE_FILES='$COMPOSE_FILES' bash ./rollout.sh"
else
  # 测试环境保持就地替换：它前面没有 nginx（不对公网开放，靠 SSH 端口转发访问），
  # 没有可切换的 upstream，蓝绿在这里既无处可切也无人受益。
  REMOTE_DEPLOY_CMD="cd $REMOTE_Q && $DC config --quiet && $DC up -d --build && for i in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30; do if curl -fsS http://127.0.0.1:${HEALTH_PORT}/health >/dev/null; then $DC ps; exit 0; fi; sleep 2; done; $DC logs --tail=100 backend; exit 1"
fi
REMOTE_DEPLOY_CMD_Q="$(printf '%q' "$REMOTE_DEPLOY_CMD")"
ssh_run "flock -w $DEPLOY_LOCK_TIMEOUT_SECS $REMOTE_LOCK_Q bash -c $REMOTE_DEPLOY_CMD_Q"
if [ "$TARGET" = "prod" ]; then
  echo "deployment healthy at https://code.mrday.one/health"
else
  echo "测试环境已就绪：ssh -L ${HEALTH_PORT}:127.0.0.1:${HEALTH_PORT} 上去之后开 http://127.0.0.1:${HEALTH_PORT}/health"
  echo "（它不对公网开放，宿主 nginx 不代理这个端口）"
fi
