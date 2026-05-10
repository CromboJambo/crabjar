#!/usr/bin/env bash
# calculate attention score for a specific event ID
set -euo pipefail

ATTENTION_DB="${ATTENTION_DB:-$HOME/.mirror-lab/mirror-log/attention.db}"
EVENT_ID="${1:-}"

if [[ -z "$EVENT_ID" ]]; then
    echo '{"error": "event ID required", "usage": "attention_score.sh <event_id"}'
    exit 1
fi

if [[ ! -f "$ATTENTION_DB" ]]; then
    echo '{"error": "attention database not found", "path": "'"$ATTENTION_DB"'"}'
    exit 1
fi

sqlite3 "$ATTENTION_DB" <<SQL
.mode json
SELECT
    (SELECT COUNT(*) FROM events WHERE id = '$EVENT_ID') AS exists,
    (SELECT importance FROM attention_items WHERE id = '$EVENT_ID') AS score,
    (SELECT last_accessed_str FROM attention_items WHERE id = '$EVENT_ID') AS last_accessed_str,
    (SELECT pinned FROM attention_items WHERE id = '$EVENT_ID') AS pinned,
    CASE
        WHEN importance >= 80 THEN 'High priority/recent'
        WHEN importance >= 50 THEN 'Medium priority'
        WHEN importance >= 20 THEN 'Low priority/stale'
        ELSE 'Cold storage'
    END AS context
SQL
