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
# code_corpus 的数据不进备份。
#
# 它是按需从各语言的包registry抓回来的 API 表面缓存（见 code_corpus.rs 顶部），丢了会自己
# 长回来——而它占了整个库 3,594MB 里的 3,509MB，97.6%。2026-08-22 实测的后果：
# 语料灌满之后单份 dump 从 13-24MB 涨到 275MB，而 deploy.sh 每次部署都先跑一遍这个脚本
# （近几天 16-29 次/天），/var/backups/michael-ide 已经 230 份 9.2GB，按这个速度约 12 天
# 把剩下的 52GB 吃光。磁盘满之后 `up --build` 会失败而旧容器继续 healthy，部署看起来是
# 成功的（docs/OPERATIONS.md「踩过的坑」记着这一幕）。排除之后单份回到 ~20MB / 十秒内，
# 异地副本也才谈得上。
#
# 用 --exclude-table-data 而不是 --exclude-table：表结构必须留在 dump 里，否则恢复出来的
# 库缺表，sqlx::migrate! 在启动时会失败。
#
# code_corpus_fetches 必须一起排除，不能只排 code_corpus。那张表是抓取台账，
# recently_attempted_eco()（code_corpus.rs）拿它判断「这个包 30 天内试过了，跳过」——
# 只恢复台账不恢复语料，播种逻辑会认为每个包都刚抓过，语料库会整整空一个月。
# 两张表都不在时，seed_all 在下次启动 45 秒后照常从头播种。
docker compose -p "$COMPOSE_PROJECT" exec -T postgres sh -c \
  'pg_dump -U "$POSTGRES_USER" -d "$POSTGRES_DB" --format=custom \
     --exclude-table-data=code_corpus --exclude-table-data=code_corpus_fetches' \
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
