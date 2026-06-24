#!/usr/bin/env bash
# Deploy the backend to your server. Target comes from env vars / args — NO
# credentials are hardcoded. Run this from a machine that can reach the server.
#
#   SERVER_HOST=103.39.67.244 SERVER_PORT=19537 SERVER_USER=root ./deploy.sh
#
set -euo pipefail

: "${SERVER_HOST:?set SERVER_HOST, e.g. SERVER_HOST=103.39.67.244}"
SERVER_PORT="${SERVER_PORT:-22}"
SERVER_USER="${SERVER_USER:-root}"
REMOTE_DIR="${REMOTE_DIR:-/opt/michael-backend}"
SSH="ssh -p ${SERVER_PORT} ${SERVER_USER}@${SERVER_HOST}"

echo "→ ensuring ${REMOTE_DIR} on ${SERVER_USER}@${SERVER_HOST}:${SERVER_PORT}"
$SSH "mkdir -p ${REMOTE_DIR}"

echo "→ syncing source (excluding target/ and .env)"
rsync -az --delete -e "ssh -p ${SERVER_PORT}" \
  --exclude target --exclude .env --exclude .git \
  ./ "${SERVER_USER}@${SERVER_HOST}:${REMOTE_DIR}/"

echo "→ checking for ${REMOTE_DIR}/.env on the server"
if ! $SSH "test -f ${REMOTE_DIR}/.env"; then
  echo "‼  No .env on the server yet. Copy .env.example → .env there and fill in"
  echo "   JWT_SECRET / POSTGRES_PASSWORD / QQ_SMTP_* before the first run."
  echo "   (Run: $SSH \"cd ${REMOTE_DIR} && cp .env.example .env && nano .env\")"
  exit 1
fi

echo "→ building & starting containers"
$SSH "cd ${REMOTE_DIR} && docker compose up -d --build && docker compose ps"
echo "✓ done — health check: curl http://${SERVER_HOST}:8080/health"
