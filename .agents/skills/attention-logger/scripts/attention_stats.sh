#!/usr/bin/env bash
# query attention layer statistics
set -euo pipefail

ATTENTION_DB="${ATTENTION_DB:-$HOME/.mirror-lab/mirror-log/attention.db}"

if [[ ! -f "$ATTENTION_DB" ]]; then
    echo '{"error": "attention database not found", "path": "'"$ATTENTION_DB"'"}'
    exit 1
fi

sqlite3 "$ATTENTION_DB" <<SQL
.mode json
SELECT
    (SELECT COUNT(*) FROM events) AS total_events,
    (SELECT COUNT(*) FROM attention_items WHERE active = 1) AS active_events,
    (SELECT COUNT(*) FROM attention_items WHERE pinned = 1) AS pinned_events,
    (SELECT COUNT(*) FROM attention_items WHERE flagged = 1) AS flagged_events,
    ROUND((SELECT COUNT(*) FROM attention_items WHERE active = 1) * 100.0 / (SELECT COUNT(*) FROM events), 2) AS active_percentage
SQL
