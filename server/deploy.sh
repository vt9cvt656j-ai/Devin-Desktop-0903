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
REMOTE_DIR="${REMOTE_DIR:-/opt/michael-ide-deploy/server}"
REMOTE="${SERVER_USER}@${SERVER_HOST}"
REMOTE_Q="$(printf '%q' "$REMOTE_DIR")"
REMOTE_LOCK="${REMOTE_LOCK:-/var/lock/michael-ide-deploy.lock}"
REMOTE_LOCK_Q="$(printf '%q' "$REMOTE_LOCK")"
DEPLOY_LOCK_TIMEOUT_SECS="${DEPLOY_LOCK_TIMEOUT_SECS:-900}"

SSH_ARGS=(-p "$SERVER_PORT" -o BatchMode=yes -o ConnectTimeout=10 -o ConnectionAttempts=3)
if [[ -n "$SERVER_KEY" ]]; then
  SSH_ARGS+=(-i "$SERVER_KEY")
fi

ssh_run() {
  local attempt status
  for attempt in 1 2 3; do
    if ssh "${SSH_ARGS[@]}" "$REMOTE" "$@"; then
      return 0
    fi
    status=$?
    sleep $((attempt * 2))
  done
  return "$status"
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
    rsync -az --delete-delay -e "$RSYNC_RSH" \
      --exclude target --exclude .env --exclude .git \
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

echo "checking for ${REMOTE_DIR}/.env on the server"
if ! ssh_run "test -f $REMOTE_Q/.env"; then
  echo "No .env exists on the server. Copy .env.example to .env and fill in"
  echo "   JWT_SECRET / POSTGRES_PASSWORD / QQ_SMTP_* before the first run."
  exit 1
fi

echo "validating, updating and health-checking containers (serialized)"
# Compose replaces the single host-published backend container in place. Two
# deploys started together can therefore stop a freshly started container and
# leave nginx with a much longer 502 window. Hold a host-side flock through the
# replacement and health check so only one rollout can touch the project at a
# time. The lock is operational coordination only; it does not change request
# handling or access policy.
REMOTE_DEPLOY_CMD="cd $REMOTE_Q && docker compose -p server config --quiet && docker compose -p server up -d --build && for i in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30; do if curl -fsS http://127.0.0.1:8080/health >/dev/null; then docker compose -p server ps; exit 0; fi; sleep 2; done; docker compose -p server logs --tail=100 backend; exit 1"
REMOTE_DEPLOY_CMD_Q="$(printf '%q' "$REMOTE_DEPLOY_CMD")"
ssh_run "flock -w $DEPLOY_LOCK_TIMEOUT_SECS $REMOTE_LOCK_Q bash -c $REMOTE_DEPLOY_CMD_Q"
echo "deployment healthy at https://code.mrday.one/health"
