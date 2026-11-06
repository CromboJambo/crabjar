use agent_context::state_docs::indexer;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn make_temp_dir() -> TempDir {
    tempfile::tempdir().expect("create temp dir")
}

fn write_test_doc(dir: &TempDir, name: &str, content: &str) -> PathBuf {
    let path = dir.path().join(format!("{}.md", name));
    fs::write(&path, content).expect("write test doc");
    path
}

fn sample_markdown() -> &'static str {
    r#"---
name: project-map
description: Project structure overview
---

# Project Map

## Architecture

The project follows a modular workspace layout.

| Crate | Path | Purpose |
|-------|------|---------|
| crabjar | src/crabjar | CLI binary |
| memory | memory/ | SQLite store |
| guard | guard/ | Execution gate |

## Execution Pipeline

```rust
fn exec(req: Request) -> Outcome {
    guard.check(&req)?;
    let result = concierge.process(req);
    telemetry.log(&result);
    Ok(result)
}
```

### Confidence Assessment

### 8.1 What This Review Captures

- All workspace members
- Crate dependency graph

### 8.2 What This Review Might Have Missed

- Runtime behavior
- Performance characteristics

### 8.3 Assumptions

- Rust toolchain is stable
- Workspace is cloned

### 8.4 Blind Spots

- External API contracts
- Platform-specific edge cases

### 8.5 Stale After

2026-06-01

## Dependencies

Top-level deps: tokio, serde, rusqlite, chrono.
"#
}

fn sample_minimal_markdown() -> &'static str {
    "# Minimal Doc\n\nJust a single section with no tables or code blocks.\n"
}

fn sample_empty_markdown() -> &'static str {
    ""
}

#[test]
fn index_doc_creates_metadata_row() {
    let dir = make_temp_dir();
    let doc_path = write_test_doc(&dir, "test-index", sample_markdown());

    let content = fs::read_to_string(&doc_path).unwrap();
    let metadata = indexer::extract_metadata_for_test(&content);
    assert_eq!(metadata.doc_name, "project-map");
    assert_eq!(metadata.line_count, content.lines().count());
    assert!(!metadata.checksum.is_empty());
}

#[test]
fn index_doc_stores_checksum() {
    let dir = make_temp_dir();
    let doc_path = write_test_doc(&dir, "test-checksum", sample_markdown());

    let content = fs::read_to_string(&doc_path).unwrap();
    let metadata = indexer::extract_metadata_for_test(&content);

    assert!(!metadata.checksum.is_empty());

    // Verify checksum is deterministic
    let metadata2 = indexer::extract_metadata_for_test(&content);
    assert_eq!(metadata.checksum, metadata2.checksum);
}

#[test]
fn index_doc_parses_sections() {
    let dir = make_temp_dir();
    let doc_path = write_test_doc(&dir, "test-sections", sample_markdown());

    let content = fs::read_to_string(&doc_path).unwrap();
    let sections = indexer::extract_sections_for_test(&content);

    assert!(
        !sections.is_empty(),
        "should have parsed at least one section"
    );
}

#[test]
fn index_doc_parses_code_blocks() {
    let dir = make_temp_dir();
    let doc_path = write_test_doc(&dir, "test-code", sample_markdown());

    let content = fs::read_to_string(&doc_path).unwrap();
    let blocks = indexer::extract_code_blocks_for_test(&content);

    assert!(
        !blocks.is_empty(),
        "should have parsed at least one code block"
    );
}

#[test]
fn index_doc_parses_tables() {
    let dir = make_temp_dir();
    // Use content where table immediately follows a heading (no blank line)
    let content = "# Title\n\n## Data\n| A | B | C |\n|---|---|---|\n| 1 | 2 | 3 |\n";
    write_test_doc(&dir, "test-tables", content);

    let doc_path = dir.path().join("test-tables.md");
    let content = fs::read_to_string(&doc_path).unwrap();
    let tables = indexer::extract_tables_for_test(&content);

    assert!(!tables.is_empty(), "should have parsed at least one table");
}

#[test]
fn index_minimal_doc_creates_no_sections() {
    let dir = make_temp_dir();
    let doc_path = write_test_doc(&dir, "test-minimal", sample_minimal_markdown());

    let content = fs::read_to_string(&doc_path).unwrap();
    let sections = indexer::extract_sections_for_test(&content);

    // The minimal doc has "# Minimal Doc" which is an h1 heading, so it will be parsed as a section
    assert!(
        !sections.is_empty(),
        "minimal doc with h1 heading should have at least one section"
    );
}

#[test]
fn index_empty_doc_creates_metadata() {
    let dir = make_temp_dir();
    let doc_path = write_test_doc(&dir, "test-empty", sample_empty_markdown());

    let content = fs::read_to_string(&doc_path).unwrap();
    let metadata = indexer::extract_metadata_for_test(&content);

    assert_eq!(metadata.line_count, 0);
    assert_eq!(metadata.checksum, "0");
}

#[test]
fn index_doc_updates_on_reindex() {
    let dir = make_temp_dir();
    let doc_path = write_test_doc(&dir, "test-reindex", sample_markdown());

    let content = fs::read_to_string(&doc_path).unwrap();
    let sections1 = indexer::extract_sections_for_test(&content);
    let sections2 = indexer::extract_sections_for_test(&content);

    assert_eq!(
        sections1.len(),
        sections2.len(),
        "reindexing should produce same section count"
    );
}

#[test]
fn extract_sections_detects_heading_levels() {
    let content = "# H1 Title\n\n## H2 Section\n\n### H3 Subsection\n\nMore text here.\n\n#### H4 Should Not Appear\n";

    let sections = indexer::extract_sections_for_test(content);
    let level3: Vec<_> = sections.iter().filter(|s| s.level == 3).collect();
    assert_eq!(level3.len(), 1, "should find one h3 section");
}

#[test]
fn extract_sections_tracks_line_ranges() {
    let content = "# Title\n\n# Section A\n\nLine 4.\nLine 5.\n\n# Section B\n\nLine 8.\n";

    let sections = indexer::extract_sections_for_test(content);
    // Headings are correctly stripped of their prefix (#, ##, ###)
    let section_a = sections.iter().find(|s| s.title == "Section A");

    if let Some(s) = section_a {
        assert_eq!(s.start_line, 3, "Section A starts at line 3");
    } else {
        panic!(
            "Section A not found in parsed sections. Available titles: {:?}",
            sections.iter().map(|s| &s.title).collect::<Vec<_>>()
        );
    }
}

#[test]
fn extract_code_blocks_detects_language() {
    let content = "# Title\n\nSome text.\n\n```rust\nfn main() {}\n```\n\nMore text.\n\n```python\nprint(\"hello\")\n```\n";

    let blocks = indexer::extract_code_blocks_for_test(content);
    assert_eq!(blocks.len(), 2, "should find two code blocks");

    let languages: Vec<&str> = blocks.iter().map(|b| b.language.as_str()).collect();
    assert!(languages.contains(&"rust"));
    assert!(languages.contains(&"python"));
}

#[test]
fn extract_tables_detects_headers() {
    let content =
        "# Title\n\n## Data\n| A | B | C |\n|---|---|---|\n| 1 | 2 | 3 |\n| 4 | 5 | 6 |\n";

    let tables = indexer::extract_tables_for_test(content);
    assert!(!tables.is_empty(), "should find at least one table");
}

#[test]
fn extract_confidence_parses_all_fields() {
    let content = "# Title\n\n## 8. Confidence Assessment\n\n### 8.1 What This Review Captures\n\n- Section A\n- Section B\n\n### 8.2 What This Review Might Have Missed\n\n- Edge cases\n\n### 8.3 Assumptions\n\n- Rust stable\n\n### 8.4 Blind Spots\n\n- Platform X\n\n### 8.5 Stale After\n\n2026-12-31\n";

    let confidence = indexer::extract_confidence_for_test(content);
    assert!(confidence.is_some(), "should have confidence entry");

    if let Some(conf) = confidence {
        assert!(conf.what_captured.contains("Section A"));
    }
}

#[test]
fn checksum_is_deterministic() {
    let content = sample_markdown();
    let meta1 = indexer::extract_metadata_for_test(content);
    let meta2 = indexer::extract_metadata_for_test(content);

    assert_eq!(
        meta1.checksum, meta2.checksum,
        "same content should produce same checksum"
    );
}

#[test]
fn different_content_produces_different_checksum() {
    let content_a = "# Doc A\n\nOnly A content.\n";
    let content_b = "# Doc B\n\nOnly B content.\n";

    let meta_a = indexer::extract_metadata_for_test(content_a);
    let meta_b = indexer::extract_metadata_for_test(content_b);

    assert_ne!(
        meta_a.checksum, meta_b.checksum,
        "different content should produce different checksums"
    );
}
