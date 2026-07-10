#!/usr/bin/env bash
set -Eeuo pipefail

umask 077

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
COMPOSE_DIR="${COMPOSE_DIR:-$SCRIPT_DIR}"
COMPOSE_PROJECT="${COMPOSE_PROJECT:-server}"
BACKUP_ROOT="${BACKUP_ROOT:-/var/backups/michael-ide}"
RETENTION_DAYS="${RETENTION_DAYS:-14}"
SITE_ROOT="${SITE_ROOT:-/var/www/michael-sites}"

mkdir -p "$BACKUP_ROOT"
exec 9>"$BACKUP_ROOT/.backup.lock"
if ! flock -n 9; then
  echo "another backup is already running" >&2
  exit 0
fi

stamp="$(date -u +%Y%m%dT%H%M%SZ)"
tmp_dir="$BACKUP_ROOT/.${stamp}.tmp"
final_dir="$BACKUP_ROOT/$stamp"
rm -rf -- "$tmp_dir"
mkdir -p "$tmp_dir"
trap 'rm -rf -- "$tmp_dir"' EXIT

cd "$COMPOSE_DIR"
docker compose -p "$COMPOSE_PROJECT" exec -T postgres sh -c \
  'pg_dump -U "$POSTGRES_USER" -d "$POSTGRES_DB" --format=custom' \
  > "$tmp_dir/postgres.dump"
test -s "$tmp_dir/postgres.dump"
docker compose -p "$COMPOSE_PROJECT" exec -T postgres pg_restore --list \
  < "$tmp_dir/postgres.dump" >/dev/null

if [[ -d "$SITE_ROOT" ]]; then
  tar --one-file-system --numeric-owner -czf "$tmp_dir/michael-sites.tar.gz" \
    -C "$(dirname "$SITE_ROOT")" "$(basename "$SITE_ROOT")"
fi

(
  cd "$tmp_dir"
  sha256sum ./* > SHA256SUMS
)
mv "$tmp_dir" "$final_dir"
trap - EXIT

find "$BACKUP_ROOT" -mindepth 1 -maxdepth 1 -type d \
  -mtime "+$RETENTION_DAYS" -exec rm -rf -- {} +

echo "backup complete: $final_dir"
