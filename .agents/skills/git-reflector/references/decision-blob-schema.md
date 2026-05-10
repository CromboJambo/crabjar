# DecisionBlob Schema

## Fields

| Field | Type | Purpose |
|---|---|---|
| `uuid` | TEXT (UUID v4) | unique identifier for this decision |
| `set_at` | ISO 8601 timestamp | moment of creation |
| `selected_reflection` | TEXT | the reflection content |
| `context_tags` | ARRAY of TEXT | tags linking to related events |
| `kernel_name` | TEXT | responsible kernel component |
| `reason` | TEXT | human-readable explanation |
| `source` | TEXT | origin path of the staged event |
| `provenance` | OBJECT | immutable creation record |

## `provenance` sub-fields

| Field | Type | Purpose |
|---|---|---|
| `created_at` | ISO 8601 timestamp | creation moment |
| `immutable` | BOOLEAN | always true — no silent overwrites |

## Constraints

- Every merge, derived output, configurable baseline gets a UUID + provenance entry
- Changes require a new provenance entry — no silent overwrites
- Adjustable baselines (thresholds, confidence defaults, decay periods) each anchored to own provenance entry
- New value replaces old via new provenance entry, not in-place mutation
