#!/usr/bin/env bash
# Puts Stripe's two secrets onto the gateway, and checks they took.
#
#   ./setup-stripe.sh
#
# Why a script rather than one long ssh line: the secrets are read from the terminal
# with echo off and handed to the server over stdin, so they never appear in a command
# line (visible to `ps`), never land in shell history, and are never written to a file on
# this machine. The only copy that persists is the one in the server's .env, which is
# root-owned and mode 600.
#
# Both values are format-checked before anything is sent. Pasting the publishable key by
# mistake, or grabbing the endpoint id instead of the signing secret, are the two easy
# errors here and both would otherwise fail much later with a confusing message.
set -euo pipefail

SERVER_HOST="${SERVER_HOST:-154.44.13.133}"
SERVER_USER="${SERVER_USER:-root}"
SERVER_KEY="${SERVER_KEY:-$HOME/.ssh/michael_server}"
REMOTE_DIR="${REMOTE_DIR:-/opt/michael-ide-deploy/server}"
REMOTE="${SERVER_USER}@${SERVER_HOST}"

SSH=(ssh -i "$SERVER_KEY" -o ConnectTimeout=30 -o ConnectionAttempts=3)

# This host drops connections mid-handshake fairly often; retry rather than fail.
retry() {
  local attempt status=0
  for attempt in 1 2 3 4 5; do
    if "$@"; then return 0; fi
    status=$?
    echo "   (connection dropped, retrying $attempt/5…)" >&2
    sleep $((attempt * 3))
  done
  return "$status"
}

echo
echo "Stripe setup for code.mrday.one"
echo "───────────────────────────────"
echo
echo "You will be asked for two values from the Stripe dashboard."
echo "Nothing you type is shown on screen. That is normal — keep typing/pasting."
echo

read -rsp "1/2  Secret key  (starts with sk_) : " SK; echo
if [[ ! "$SK" =~ ^sk_(live|test)_ ]]; then
  echo
  echo "✗ That does not look like a secret key."
  echo "  It must begin with sk_live_ or sk_test_."
  echo "  If yours begins with pk_ that is the publishable key — the gateway does not use it."
  exit 1
fi

read -rsp "2/2  Webhook signing secret  (starts with whsec_) : " WH; echo
if [[ ! "$WH" =~ ^whsec_ ]]; then
  echo
  echo "✗ That does not look like a signing secret."
  echo "  It must begin with whsec_."
  echo "  If yours begins with we_ that is the endpoint's id, not its signing secret —"
  echo "  open the endpoint in Stripe and click 'reveal' under Signing secret."
  exit 1
fi

echo
echo "→ Sending to the server…"
# Over stdin, so neither value ever appears in a command line.
printf '%s\n%s\n' "$SK" "$WH" | retry "${SSH[@]}" "$REMOTE" "
  set -e
  cd '$REMOTE_DIR'
  read -r SK
  read -r WH
  cp .env .env.before-stripe
  sed -i '/^STRIPE_SECRET_KEY=/d; /^STRIPE_WEBHOOK_SECRET=/d' .env
  printf 'STRIPE_SECRET_KEY=%s\nSTRIPE_WEBHOOK_SECRET=%s\n' \"\$SK\" \"\$WH\" >> .env
  chmod 600 .env
"
unset SK WH

echo "→ Restarting the gateway…"
retry "${SSH[@]}" "$REMOTE" "cd '$REMOTE_DIR' && docker compose -p server up -d >/dev/null 2>&1"

echo "→ Waiting for it to come back…"
retry "${SSH[@]}" "$REMOTE" "
  for i in \$(seq 1 30); do
    if curl -fsS http://127.0.0.1:8080/health >/dev/null 2>&1; then exit 0; fi
    sleep 2
  done
  exit 1
"

echo "→ Checking…"
retry "${SSH[@]}" "$REMOTE" "
  cd '$REMOTE_DIR'
  # Names and shapes only — never the values.
  grep -q '^STRIPE_SECRET_KEY=sk_' .env    && echo '   ✓ secret key stored'    || { echo '   ✗ secret key missing'; exit 1; }
  grep -q '^STRIPE_WEBHOOK_SECRET=whsec_' .env && echo '   ✓ signing secret stored' || { echo '   ✗ signing secret missing'; exit 1; }
  grep -q '^STRIPE_SECRET_KEY=sk_live_' .env && echo '   ✓ live mode (real charges)' || echo '   • test mode (no real charges)'
"

echo
echo "Done. Now reload  https://code.mrday.one/billing"
echo "The red 'card payments not switched on' banner should be gone and the"
echo "Subscribe buttons should be clickable."
echo
echo "If anything looks wrong, this puts it back exactly as it was:"
echo "  ssh -i $SERVER_KEY $REMOTE 'cd $REMOTE_DIR && cp .env.before-stripe .env && docker compose -p server up -d'"
echo
