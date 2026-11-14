#!/usr/bin/env bash
# Nightly logical backup of the NockNock database (run by nocknock-backup.timer).
#
# The database holds records that exist nowhere else in this form: the accumulation
# ledger (receipt-verified buys), the harvest ledger and the snapshot history. Everything
# in the ledgers can be re-derived from the chain, but only by someone who knows which
# transactions to look for, so a dump is kept.
#
# Dumps land in ~/nocknock/backups as nocknock-YYYY-MM-DD.sql.gz; the newest
# NOCKNOCK_BACKUP_KEEP (default 14) are kept.
set -euo pipefail

DIR="${NOCKNOCK_BACKUP_DIR:-$HOME/nocknock/backups}"
KEEP="${NOCKNOCK_BACKUP_KEEP:-14}"
mkdir -p "$DIR"
chmod 700 "$DIR"

out="$DIR/nocknock-$(date +%F).sql.gz"
tmp="$out.partial"

# Dump inside the database container; only replace a previous dump once this one is complete.
podman exec nocknock-db pg_dump -U postgres -d nocknock | gzip > "$tmp"
if [ ! -s "$tmp" ] || ! gzip -t "$tmp"; then
  echo "backup failed: dump is empty or corrupt" >&2
  exit 1
fi
mv -f "$tmp" "$out"
chmod 600 "$out"

# Retention: keep the newest $KEEP dumps.
ls -1t "$DIR"/nocknock-*.sql.gz 2>/dev/null | tail -n +"$((KEEP + 1))" | while read -r old; do
  unlink "$old"
done

echo "backup ok: $out ($(du -h "$out" | cut -f1)); $(ls -1 "$DIR"/nocknock-*.sql.gz | wc -l | tr -d ' ') kept"
