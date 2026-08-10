#!/usr/bin/env bash
# Publish the sign-in page (code.mrday.one/gate) to the server.
#
#   SERVER_KEY=~/.ssh/michael_server ./deploy-gate.sh
#
# deploy.sh ships the backend containers and deploy-account-ui.sh ships the console.
# Neither touches this file, which is why it was being copied by hand — and why an edit
# to ide/gate/gate.html could sit in the repo looking deployed while the server served
# something else entirely. nginx reads it from WEB_ROOT below (`location = /gate`).
#
# The source of truth is ide/gate/gate.html in the repo. Editing the copy on the server
# is how the good version got lost once already: it is not tracked anywhere, so the next
# person to run this script silently reverts it.
#
# Same two traps as deploy-account-ui.sh:
#
#   1. Files copied over ssh arrive owned by root with mode 640. nginx runs as www-data
#      and cannot read them, so /gate answers 403. Ownership is set explicitly.
#   2. The live copy is kept for rollback before the new one is installed.
set -euo pipefail

SERVER_HOST="${SERVER_HOST:-154.44.13.133}"
SERVER_USER="${SERVER_USER:-root}"
SERVER_KEY="${SERVER_KEY:-$HOME/.ssh/michael_server}"
WEB_ROOT="${WEB_ROOT:-/var/www/michael-gate}"
STAGE_DIR="${STAGE_DIR:-/root/gate-deploy}"
REMOTE="${SERVER_USER}@${SERVER_HOST}"

SRC="$(cd "$(dirname "$0")/.." && pwd)/ide/gate/gate.html"

SSH_BIN=(ssh -i "$SERVER_KEY" -o ConnectTimeout=30 -o ConnectionAttempts=3)
SCP_BIN=(scp -i "$SERVER_KEY" -o ConnectTimeout=30 -o ConnectionAttempts=3)

# This host drops connections during the handshake often enough to fail a deploy halfway
# ("banner exchange: ... invalid format"). ConnectionAttempts does not cover it — the TCP
# connection succeeds and then dies — so retry the whole command.
retry() {
  local attempt status=0
  for attempt in 1 2 3 4 5; do
    if "$@"; then
      return 0
    fi
    status=$?
    echo "    (attempt $attempt failed, retrying in $((attempt * 3))s)" >&2
    sleep $((attempt * 3))
  done
  return "$status"
}

SSH=(retry "${SSH_BIN[@]}")
SCP=(retry "${SCP_BIN[@]}")

[ -f "$SRC" ] || { echo "missing $SRC"; exit 1; }

# The page is one self-contained file with inline script, so "does it parse" is the only
# build step there is. A syntax error here takes sign-in down for everyone.
if command -v node >/dev/null 2>&1; then
  echo "==> checking the inline script parses"
  node -e '
    const fs = require("fs");
    const html = fs.readFileSync(process.argv[1], "utf8");
    const src = [...html.matchAll(/<script[^>]*>([\s\S]*?)<\/script>/g)].map((m) => m[1]).join("\n");
    new Function(src);
  ' "$SRC"
fi

echo "==> uploading"
"${SSH[@]}" "$REMOTE" "mkdir -p $WEB_ROOT $STAGE_DIR"
"${SCP[@]}" -q "$SRC" "$REMOTE:$STAGE_DIR/gate.html"

echo "==> keeping the live copy for rollback, then installing"
"${SSH[@]}" "$REMOTE" "cp -a $WEB_ROOT/gate.html $STAGE_DIR/gate.html.live-backup 2>/dev/null || true"
"${SSH[@]}" "$REMOTE" "install -m 0644 -o www-data -g www-data $STAGE_DIR/gate.html $WEB_ROOT/gate.html"

echo "==> verifying"
"${SSH[@]}" "$REMOTE" "sudo -u www-data test -r $WEB_ROOT/gate.html || { echo 'nginx cannot read gate.html'; exit 1; }"
curl -fsS -o /dev/null -w "    https://code.mrday.one/gate -> HTTP %{http_code}\n" https://code.mrday.one/gate

echo
echo "deployed. roll back with:"
echo "  ssh -i $SERVER_KEY $REMOTE 'install -m 0644 -o www-data -g www-data $STAGE_DIR/gate.html.live-backup $WEB_ROOT/gate.html'"
