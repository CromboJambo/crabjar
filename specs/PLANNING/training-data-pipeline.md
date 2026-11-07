# Training Data Pipeline: File Watcher, Secret Stripper, Dedup Logic

**Status:** Proposed  
**Date:** October 28, 2026  
**Author:** Agent (scoped from user direction)

## Problem Statement

Training data is scattered across sessions and machines. We need a centralized pipeline that:
1. Watches a dump directory for new training material
2. Strips secrets (SSH keys, tokens, emails, IPs)
3. Deduplicates content while tracking occurrence frequency
4. Produces clean, ready-to-train data

## Design Principles

1. **Reuse proven patterns**: mirror-lab's SQLite state management, health checks, and schema migration approach
2. **Idempotent processing**: Running the pipeline multiple times produces identical results
3. **Occurrence tracking**: High-frequency items are valuable training signals (recurrent patterns)
4. **Secret safety**: Default to over-stripping rather than leaking

## Architecture

```
raw/sessions/ → [File Watcher] → [Secret Stripper] → [Dedup Engine] → processed/clean/
     ↓                ↓                  ↓                 ↓
  inotify        SHA-256 hash      regex patterns    registry.json
```

### Component 1: File Watcher Daemon

**Technology:** Rust with `inotify` crate (Linux) or polling fallback  
**Inspiration:** mirror-lab's service health check daemon pattern

Responsibilities:
- Monitor `raw/` directory for new files
- Queue files for processing (FIFO)
- Handle concurrent file additions
- Report processing status via health endpoint

Implementation approach:
```hemlock
// Pseudocode — actual implementation in Rust
let watcher = InotifyWatcher::new("/home/crombo/training-data-dump/raw/");
loop {
    let event = watcher.wait();
    if (event.is_create) {
        queue_for_processing(event.path);
    }
}
```

### Component 2: Secret Stripper Tool

**Technology:** Rust binary using `regex` crate  
**Patterns to detect:**
- SSH keys: `ssh-ed25519 AAAA...`, `ssh-rsa AAAA...`
- API tokens: long alphanumeric strings (32+ chars)
- Email addresses: standard regex
- IP addresses: IPv4/IPv6 patterns (configurable)
- File paths with usernames: `/home/<user>/...`

Replacement strategy: `<SECRET:type>` placeholders  
Example: `ssh-ed25519 AAAA...xyz` → `<SECRET:ssh_key>`

Implementation approach:
```hemlock
let patterns = [
    { type: "ssh_key", regex: "ssh-(ed25519|rsa)\\s+[A-Za-z0-9+/=]+" },
    { type: "email", regex: "[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}" },
    // ... more patterns
];

for (pattern in patterns) {
    text = text.replace(pattern.regex, `<SECRET:${pattern.type}>`);
}
```

### Component 3: Dedup Engine

**Technology:** Rust with SQLite backend (reusing mirror-lab's state management pattern)  
**Algorithm:** SHA-256 content hashing after normalization

Normalization steps:
1. Collapse whitespace
2. Case-fold for comparison (but preserve original case in output)
3. Strip leading/trailing whitespace

Registry schema (SQLite):
```sql
CREATE TABLE training_entries (
    id INTEGER PRIMARY KEY,
    content_hash TEXT UNIQUE NOT NULL,
    original_path TEXT NOT NULL,
    processed_path TEXT,
    first_seen TIMESTAMP NOT NULL,
    last_seen TIMESTAMP NOT NULL,
    occurrences INTEGER DEFAULT 1,
    secrets_found TEXT,  -- JSON array of secret types found
    status TEXT NOT NULL DEFAULT 'pending'  -- pending/clean/duplicate/error
);

CREATE INDEX idx_hash ON training_entries(content_hash);
CREATE INDEX idx_status ON training_entries(status);
```

Processing logic:
```hemlock
let hash = sha256(normalize(text));
let entry = db.query("SELECT * FROM training_entries WHERE content_hash = ?", [hash]);

if (entry) {
    // Duplicate — increment occurrence count
    db.execute("UPDATE training_entries SET occurrences = occurrences + 1, last_seen = ? WHERE content_hash = ?", 
               [now(), hash]);
} else {
    // New entry — insert and process
    db.execute("INSERT INTO training_entries (content_hash, original_path, first_seen, last_seen) VALUES (?, ?, ?, ?)",
               [hash, path, now(), now()]);
    let cleaned = strip_secrets(text);
    write_file(processed_dir + hash + ".md", cleaned);
}
```

## Implementation Plan

### Phase 1: Secret Stripper Tool (Week 1)
- Standalone Rust binary that reads input file and writes cleaned output
- Configurable patterns via JSON config file
- Test suite with known secret formats

### Phase 2: Dedup Engine (Week 2)
- SQLite-backed deduplication with occurrence tracking
- CLI interface for manual processing
- Registry export/import functionality

### Phase 3: File Watcher Daemon (Week 3)
- Inotify-based directory monitoring
- Integration with secret stripper and dedup engine
- Health check endpoint for mirror-lab monitoring

## Testing Strategy

1. **Secret stripper**: Test corpus with known secrets, verify all are stripped
2. **Dedup engine**: Feed duplicate content, verify occurrence counting
3. **File watcher**: Drop files into monitored directory, verify processing pipeline

## Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| Over-stripping legitimate content | Configurable patterns, whitelist option |
| Hash collisions | Use SHA-256 (negligible collision probability) |
| Daemon crashes | Health checks restart it, state is persisted in SQLite |
| Secret pattern updates | Patterns stored in config file, hot-reloadable |

## References

- mirror-lab state management: `~/projects/active/mirror-lab/` — SQLite schema patterns, health check daemon architecture
- Training data dump location: `/home/crombo/training-data-dump/`
- Session export format: Markdown with structured metadata
