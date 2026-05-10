#!/usr/bin/env bash
# inspect active items from mirror-log attention layer
set -euo pipefail

ATTENTION_DB="${ATTENTION_DB:-$HOME/.mirror-lab/mirror-log/attention.db}"
OUTPUT_FORMAT="${OUTPUT_FORMAT:-json}"

if [[ ! -f "$ATTENTION_DB" ]]; then
    echo '{"error": "attention database not found", "path": "'"$ATTENTION_DB"'"}'
    exit 1
fi

sqlite3 "$ATTENTION_DB" <<SQL
.mode json
SELECT id, source, content_snippet, last_accessed, last_accessed_str, pinned, access_count
FROM attention_items
WHERE active = 1
ORDER BY importance DESC;
SQL
