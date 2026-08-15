#!/usr/bin/env bash
# Publish the admin console (code.mrday.one:8443/console/) to the server.
#
#   SERVER_KEY=~/.ssh/michael_server ./deploy-admin-ui.sh
#
# deploy.sh next to this file ships the backend containers; it does not touch the
# frontends. This script is the missing half for admin-ui.
#
# Two traps it exists to close, both hit on 2026-08-04:
#
#   1. Files copied to the server over ssh arrive owned by root with mode 640. nginx
#      runs as www-data and cannot read them, so the page loads with no styling at
#      all. Ownership is set explicitly below, before anything points at the files.
#
#   2. index.html is the switch: it names the hashed bundle to load. Upload it LAST.
#      Upload it first and every request in between asks for a file that is not there
#      yet. Assets are content-hashed, so old and new can sit side by side safely.
set -euo pipefail

SERVER_HOST="${SERVER_HOST:-154.44.13.133}"
SERVER_USER="${SERVER_USER:-root}"
SERVER_KEY="${SERVER_KEY:-$HOME/.ssh/michael_server}"
WEB_ROOT="${WEB_ROOT:-/var/www/michael-console}"
STAGE_DIR="${STAGE_DIR:-/root/console-deploy}"
REMOTE="${SERVER_USER}@${SERVER_HOST}"

SSH_BIN=(ssh -i "$SERVER_KEY" -o ConnectTimeout=30 -o ConnectionAttempts=3)
SCP_BIN=(scp -i "$SERVER_KEY" -o ConnectTimeout=30 -o ConnectionAttempts=3)

# This host drops connections during the handshake often enough to fail a deploy
# halfway ("banner exchange: ... invalid format"). ConnectionAttempts does not cover
# it — the TCP connection succeeds and then dies — so retry the whole command.
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

cd "$(dirname "$0")/admin-ui"

# Clean build: see the note in .gitignore about Tailwind scanning a stale dist.
echo "==> building (clean)"
rm -rf dist
npm run build

CSS="$(cd dist/assets && ls ./*.css | head -1 | sed 's|^\./||')"
JS="$(cd dist/assets && ls ./*.js | head -1 | sed 's|^\./||')"
echo "    bundle: $CSS  $JS"

# Sanity check: index.html must reference exactly the files we are about to upload.
for f in "$CSS" "$JS"; do
  grep -q "$f" dist/index.html || { echo "index.html does not reference $f — aborting"; exit 1; }
done

echo "==> uploading assets and fonts"
"${SSH[@]}" "$REMOTE" "mkdir -p $WEB_ROOT/assets $WEB_ROOT/fonts $STAGE_DIR"
"${SCP[@]}" "dist/assets/$CSS" "dist/assets/$JS" "$REMOTE:$WEB_ROOT/assets/"
"${SCP[@]}" -q dist/fonts/* "$REMOTE:$WEB_ROOT/fonts/"
"${SCP[@]}" -q dist/logo.png "$REMOTE:$WEB_ROOT/logo.png"

# Trap 1: make everything readable by nginx BEFORE index.html points at it.
echo "==> handing the files to nginx (www-data)"
"${SSH[@]}" "$REMOTE" "chown -R www-data:www-data $WEB_ROOT && chmod -R u=rwX,go=rX $WEB_ROOT"

# Trap 2: index.html last, staged then installed, with the live copy kept for rollback.
echo "==> switching index.html over"
"${SCP[@]}" -q dist/index.html "$REMOTE:$STAGE_DIR/index.html"
"${SSH[@]}" "$REMOTE" "cp -a $WEB_ROOT/index.html $STAGE_DIR/index.html.live-backup 2>/dev/null || true"
"${SSH[@]}" "$REMOTE" "install -m 0644 -o www-data -g www-data $STAGE_DIR/index.html $WEB_ROOT/index.html"

echo "==> verifying"
"${SSH[@]}" "$REMOTE" "
  set -e
  grep -q '$CSS' $WEB_ROOT/index.html || { echo 'index.html does not name the new css'; exit 1; }
  sudo -u www-data test -r $WEB_ROOT/assets/$CSS || { echo 'nginx cannot read the css'; exit 1; }
  sudo -u www-data test -r $WEB_ROOT/assets/$JS  || { echo 'nginx cannot read the js';  exit 1; }
  sudo -u www-data test -r $WEB_ROOT/index.html  || { echo 'nginx cannot read index.html'; exit 1; }
"

echo
echo "deployed. /billing and /dashboard are served no-store, so a normal reload picks it up."
echo "roll back with:"
echo "  ssh -i $SERVER_KEY $REMOTE 'install -m 0644 -o www-data -g www-data $STAGE_DIR/index.html.live-backup $WEB_ROOT/index.html'"
