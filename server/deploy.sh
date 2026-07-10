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

SSH_ARGS=(-p "$SERVER_PORT" -o BatchMode=yes)
if [[ -n "$SERVER_KEY" ]]; then
  SSH_ARGS+=(-i "$SERVER_KEY")
fi

ssh_run() {
  ssh "${SSH_ARGS[@]}" "$REMOTE" "$@"
}

RSYNC_RSH="ssh -p $(printf '%q' "$SERVER_PORT") -o BatchMode=yes"
if [[ -n "$SERVER_KEY" ]]; then
  RSYNC_RSH+=" -i $(printf '%q' "$SERVER_KEY")"
fi

echo "ensuring ${REMOTE_DIR} on ${REMOTE}:${SERVER_PORT}"
ssh_run "mkdir -p $REMOTE_Q"

echo "creating a pre-deploy backup when the installed backup script is available"
ssh_run "if test -x $REMOTE_Q/backup.sh; then $REMOTE_Q/backup.sh; fi"

echo "syncing source (excluding build output and .env)"
rsync -az --delete-delay -e "$RSYNC_RSH" \
  --exclude target --exclude .env --exclude .git \
  --exclude '*.bak' --exclude '*.bak.*' --exclude '*.pre-*' \
  ./ "$REMOTE:$REMOTE_DIR/"

echo "checking for ${REMOTE_DIR}/.env on the server"
if ! ssh_run "test -f $REMOTE_Q/.env"; then
  echo "No .env exists on the server. Copy .env.example to .env and fill in"
  echo "   JWT_SECRET / POSTGRES_PASSWORD / QQ_SMTP_* before the first run."
  exit 1
fi

echo "validating and starting containers"
ssh_run "cd $REMOTE_Q && docker compose -p server config --quiet && docker compose -p server up -d --build"

echo "waiting for the loopback health endpoint"
ssh_run "for i in \$(seq 1 30); do if curl -fsS http://127.0.0.1:8080/health >/dev/null; then docker compose -p server ps; exit 0; fi; sleep 2; done; docker compose -p server logs --tail=100 backend; exit 1"
echo "deployment healthy at https://code.mrday.one/health"
