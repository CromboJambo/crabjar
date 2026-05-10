# Attention Layer Schema

## `attention_items` table

| Column | Type | Purpose |
|---|---|---|
| `id` | TEXT (UUID) | unique event identifier |
| `source` | TEXT | origin of the event (file path, tool call, etc.) |
| `content_snippet` | TEXT | trimmed content representation |
| `last_accessed` | INTEGER (Unix timestamp) | timestamp of last access |
| `last_accessed_str` | TEXT | human-readable duration (e.g., "2h ago") |
| `pinned` | INTEGER (0/1) | immune to decay flag |
| `access_count` | INTEGER | number of accesses since creation |
| `active` | INTEGER (0/1) | currently in active thought |
| `flagged` | INTEGER (0/1) | approaching decay threshold |
| `importance` | REAL | computed attention score |

## `events` table

| Column | Type | Purpose |
|---|---|---|
| `id` | TEXT (UUID) | unique event identifier |
| `source` | TEXT | origin |
| `content` | TEXT | full event content |
| `created_at` | INTEGER (Unix timestamp) | creation timestamp |

## Decay calculation

`importance = f(access_count, last_accessed, pinned)`

- `pinned = 1` → importance = 100 (constant)
- `pinned = 0` → importance decays based on time since last access and access frequency
- `flagged = 1` when importance drops below decay threshold (configurable, default 20)

## Query patterns

- **Active items**: `WHERE active = 1 ORDER BY importance DESC`
- **Flagged items**: `WHERE flagged = 1 ORDER BY importance ASC`
- **Pinned items**: `WHERE pinned = 1`
- **Single score**: `SELECT importance FROM attention_items WHERE id = '<event_id>'`
