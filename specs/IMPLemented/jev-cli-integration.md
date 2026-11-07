# JEV CLI Integration Specification

**Status:** Proposed  
**Last Updated:** 2026-10-17  
**Author:** crabjar team (CromboJambo)

## Overview

Design for the `crabjar decide` subcommand that exposes JEV semantic decision-making via embedding similarity. The CLI provides three operations: evaluate observations against criteria, list available criteria libraries, and validate criteria files.

This spec extends the existing `Decide` variant in `src/lib.rs` with proper clap subcommands, integrates with the llama-server deployment on ftw3 (http://localhost:8080), and follows crabjar's CLI output contract (structured JSON with doubt block).

## Architecture

### Command Hierarchy

```
crabjar decide <subcommand> [args]
├── evaluate <observation> --criteria <file|inline> [--backend <provider>] [--threshold <f64>]
├── list-criteria [--path <dir>]
└── validate <criteria.json>
```

### Backend Abstraction

The CLI supports multiple embedding backends via the `--backend` flag:

- `llama-server` (default): HTTP backend to llama-server at http://localhost:8080 using /api/v1/embeddings endpoint
- `hash`: Local hash-based embedding for testing/offline use (existing implementation in semantic.rs)
- `candle`: Future Candle-based local inference backend

### Module Structure

```
src/commands/
├── decide.rs              # CLI command parsing and dispatch (~200 LoC)
├── decide_backend.rs      # EmbeddingBackend trait + implementations (~350 LoC)
└── decide_criteria.rs     # Criteria loading, validation, library management (~400 LoC)
```

Each module stays within the 500 LoC CI gate. The existing `semantic.rs` is retained as a thin compatibility layer that delegates to these modules.

### Data Flow

```
CLI args → decide.rs (parse) → decide_criteria.rs (load criteria) 
         → decide_backend.rs (embed observation + criteria)
         → semantic::evaluate() (cosine similarity comparison)
         → structured JSON response with doubt block
```

## CLI Design

### `crabjar decide evaluate`

Evaluate an observation against decision criteria and return the best matching outcome.

**Usage:**
```bash
crabjar decide evaluate "service homeassistant not responding on port 8123" \
    --criteria /opt/crabjar/criteria/service-triage.json \
    --backend llama-server \
    --threshold 0.75
```

**Options:**
- `<observation>`: Free-text observation to evaluate (required)
- `--criteria <path|inline>`: Path to criteria JSON file, or inline JSON string prefixed with `@` (required)
- `--backend <provider>`: Embedding backend (default: `llama-server`)
- `--threshold <f64>`: Confidence threshold for decisive outcomes (default: 0.75)
- `--verbose`: Include full similarity scores for all options

**Success Response:**
```json
{
    "success": true,
    "type": "decision",
    "observation": "service homeassistant not responding on port 8123",
    "criteria_source": "/opt/crabjar/criteria/service-triage.json",
    "backend": "llama-server",
    "result": {
        "option": "restart_service",
        "confidence": 0.847,
        "rationale": "High similarity to service-down criterion (0.847 vs threshold 0.75)"
    },
    "doubt": {
        "assumptions": [
            "Embedding backend available at http://localhost:8080",
            "Criteria file is valid and contains distinct option embeddings"
        ],
        "blind_spots": [
            "Confidence threshold is heuristic; no historical precision/recall data available"
        ],
        "last_validation": "2026-10-17T15:30:00Z",
        "stale_after": "2026-10-18T15:30:00Z"
    }
}
```

**Uncertain Response (below threshold):**
```json
{
    "success": true,
    "type": "decision",
    "observation": "...",
    "result": {
        "option": null,
        "confidence": 0.623,
        "rationale": "Best match below threshold (0.623 < 0.75); route to human review"
    },
    "doubt": { ... }
}
```

**Error Response:**
```json
{
    "success": false,
    "type": "decision_error",
    "error": "Backend unreachable at http://localhost:8080: connection refused",
    "usage": ["crabjar decide evaluate <observation> --criteria <file|inline>"]
}
```

### `crabjar decide list-criteria`

List available decision criteria libraries.

**Usage:**
```bash
crabjar decide list-criteria
crabjar decide list-criteria --path /opt/crabjar/criteria
```

**Success Response:**
```json
{
    "success": true,
    "type": "criteria_list",
    "search_paths": ["/opt/crabjar/criteria", "~/.crabjar/criteria"],
    "libraries": [
        {
            "name": "service-triage",
            "path": "/opt/crabjar/criteria/service-triage.json",
            "options": ["restart_service", "disk_full", "network_issue", "ignore"],
            "version": "1.2.0"
        },
        {
            "name": "deployment-gate",
            "path": "/opt/crabjar/criteria/deployment-gate.json",
            "options": ["proceed", "hold", "abort"],
            "version": "1.0.3"
        }
    ],
    "count": 2,
    "doubt": { ... }
}
```

### `crabjar decide validate`

Validate a criteria file for structural correctness and semantic distinctness.

**Usage:**
```bash
crabjar decide validate /opt/crabjar/criteria/my-criteria.json
```

**Success Response (valid):**
```json
{
    "success": true,
    "type": "criteria_validation",
    "file": "/opt/crabjar/criteria/my-criteria.json",
    "valid": true,
    "checks": {
        "schema_valid": true,
        "options_distinct": true,
        "min_similarity_ok": 0.312
    },
    "doubt": { ... }
}
```

**Error Response (invalid):**
```json
{
    "success": false,
    "type": "criteria_validation",
    "file": "/opt/crabjar/criteria/my-criteria.json",
    "valid": false,
    "errors": [
        "Options 'urgent' and 'critical' are too similar (cosine: 0.91); consider merging or rewording"
    ],
    "doubt": { ... }
}
```

## Implementation Details

### Backend Trait (`decide_backend.rs`)

```hemlock
trait EmbeddingBackend {
    fn name() -> &'static str;
    fn embed(&self, text: &str) -> Result<Vec<f64>, BackendError>;
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f64>>, BackendError>;
}

impl EmbeddingBackend for LlamaServerBackend {
    fn name() -> &'static str { "llama-server" }
    
    fn embed(&self, text: &str) -> Result<Vec<f64>, BackendError> {
        let body = json!({
            "model": self.model_id,
            "input": text
        });
        let response = reqwest::Client::new()
            .post(format!("{}/api/v1/embeddings", self.url))
            .json(&body)
            .send()?
            .json::<EmbeddingResponse>()?;
        Ok(response.data[0].embedding)
    }
}
```

### Criteria Loading (`decide_criteria.rs`)

Criteria files use the standard JEV JSON format with schema validation:

```hemlock
pub struct DecisionCriteria {
    pub version: String,
    pub options: Vec<CriterionOption>,
    pub threshold: f64,
}

pub struct CriterionOption {
    pub id: String,
    pub description: String,
    // Embedding computed at evaluation time by backend
}
```

Discovery follows crabjar's config hierarchy:
1. Explicit `--criteria` path (highest priority)
2. `/opt/crabjar/criteria/<name>.json` (system install)
3. `~/.crabjar/criteria/<name>.json` (user override)

### Integration with Existing CLI Infrastructure

The existing `Decide` variant in `src/lib.rs` is updated to use proper clap subcommands:

```hemlock
// Before (current): positional args parsed manually
pub enum CliCommand {
    Decide {
        #[arg(default_value = "evaluate")]
        subcmd: String,
        observation: Option<String>,
        criteria_file: Option<String>,
    },
}

// After (this spec): proper subcommand enum
pub enum CliCommand {
    Decide {
        #[command(subcommand)]
        command: DecideCommand,
    },
}

#[derive(Debug, Subcommand, Clone)]
pub enum DecideCommand {
    Evaluate {
        observation: String,
        #[arg(long)]
        criteria: String,
        #[arg(long, default_value = "llama-server")]
        backend: String,
        #[arg(long, default_value_t = 0.75)]
        threshold: f64,
    },
    ListCriteria {
        #[arg(long)]
        path: Option<String>,
    },
    Validate {
        file: String,
    },
}
```

### Error Handling Strategy

| Error Condition | HTTP Status | JSON `success` | Notes |
|-----------------|-------------|----------------|-------|
| Backend unreachable | N/A (CLI) | false | Include endpoint and error details |
| Criteria file not found | N/A (CLI) | false | Suggest available libraries |
| Invalid criteria schema | N/A (CLI) | false | List specific validation errors |
| Observation too long (>8k chars) | N/A (CLI) | false | Truncation warning in doubt block |
| Timeout (default 30s) | N/A (CLI) | false | Include elapsed time |

## Testing Strategy

### Unit Tests (`src/commands/decide.rs`)

- `evaluate_command_parses_args`: Verify clap parsing of all option combinations
- `list_criteria_empty_dir`: Return empty array when no criteria files found
- `validate_catches_schema_errors`: Detect missing required fields in criteria JSON
- `threshold_default_applied`: Default 0.75 threshold used when not specified

### Integration Tests (`tests/cli/decide.rs`)

- `evaluate_with_hash_backend`: End-to-end evaluation using hash backend (no network)
- `evaluate_with_llama_server`: Real evaluation against deployed llama-server (requires http://localhost:8080)
- `list_criteria_finds_system_libraries`: Discovery of criteria in /opt/crabjar/criteria
- `validate_roundtrip`: Generate valid criteria → validate → evaluate

### Snapshot Tests (`tests/snapshots/`)

- CLI JSON output format for success, error, and uncertain responses
- Doubt block structure and field names
- Criteria list response with multiple libraries

### Test Fixtures

```
tests/fixtures/criteria/
├── valid-service-triage.json      # Standard 4-option criteria
├── invalid-missing-options.json   # Schema validation test
└── overlapping-criteria.json      # Similarity check test
```

## Deployment Notes

1. Build and deploy binary to ftw3: `cargo build --release && scp target/release/crabjar crombo@ftw3:/opt/crabjar/crabjar`
2. Ensure llama-server is running on ftw3 with Qwen3.5-4B-Q4_K_S model at http://localhost:8080
3. Place criteria libraries in `/opt/crabjar/criteria/` on ftw3
4. Update dinit service definition if needed to expose port 8080

## Future Work

- `crabjar decide learn`: Feed historical outcomes back to adjust thresholds dynamically
- Integration with knowledge store confidence scores for learned threshold calibration
- Distributional output: return probability distribution across all options, not just best match
- Caching layer for repeated observations (embedding cache keyed by observation hash)
