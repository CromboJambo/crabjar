// crabjar/memory/src/state_docs/extract.rs
// Markdown parsing — extract sections, tables, code blocks, confidence from state-docs.

use crate::state_docs::models::{CodeBlock, ConfidenceAssessment, DocMetadata, Section, Table};
use chrono::Utc;

/// Extract metadata from the frontmatter of a Markdown file
pub fn extract_metadata(content: &str) -> DocMetadata {
    let mut doc_name = String::new();
    let mut description = String::new();

    // Parse YAML frontmatter (--- delimited)
    if content.starts_with("---") {
        let parts: Vec<&str> = content.split("---").collect();
        if parts.len() >= 3 {
            let frontmatter = parts[1];
            for line in frontmatter.lines() {
                if let Some(key_val) = line.split_once(':') {
                    let key = key_val.0.trim();
                    let val = key_val.1.trim();
                    match key {
                        "name" => doc_name = val.to_string(),
                        "description" => description = val.to_string(),
                        _ => {}
                    }
                }
            }
        }
    }

    // Extract display_name from first h1
    let display_name = content
        .lines()
        .find(|line| line.starts_with("# "))
        .map(|line| line.trim_start_matches("# ").trim())
        .unwrap_or("Untitled");

    DocMetadata {
        doc_name,
        display_name: display_name.to_string(),
        description,
        path: doc_path_to_string(content),
        last_modified: Utc::now(),
        line_count: content.lines().count(),
        section_count: 0,
        table_count: 0,
        code_block_count: 0,
        annotation_count: 0,
        open_annotation_count: 0,
        checksum: compute_checksum(content),
    }
}

fn compute_checksum(content: &str) -> String {
    let mut hash = 0u64;
    for byte in content.bytes() {
        hash = hash.wrapping_add(byte as u64).wrapping_mul(31);
    }
    format!("{:x}", hash)
}

fn doc_path_to_string(content: &str) -> String {
    // Extract path from frontmatter if available, otherwise use content as hint
    if content.starts_with("---") {
        let parts: Vec<&str> = content.split("---").collect();
        if parts.len() >= 3 {
            let frontmatter = parts[1];
            for line in frontmatter.lines() {
                if let Some(("path", val)) = line.split_once(':') {
                    return val.trim().to_string();
                }
            }
        }
    }
    "unknown".to_string()
}

/// Extract sections (h1, h2, h3) with line ranges
pub fn extract_sections(content: &str) -> Vec<Section> {
    let lines: Vec<&str> = content.lines().collect();
    let mut sections = Vec::new();
    let mut current_section: Option<Section> = None;
    let mut section_id_counter = 1i64;

    for (i, line) in lines.iter().enumerate() {
        let line_num = i + 1; // 1-indexed

        // Detect heading level by counting leading '#'
        let heading_level = if line.starts_with("### ") {
            Some(3)
        } else if line.starts_with("## ") {
            Some(2)
        } else if line.starts_with("# ") {
            Some(1)
        } else {
            None
        };

        if let Some(level) = heading_level {
            // Close previous section
            if let Some(mut s) = current_section.take() {
                s.end_line = line_num - 1;
                sections.push(s);
            }

            // Determine parent: find the most recent section at a lower level
            let parent_id = sections
                .iter()
                .rev()
                .find(|s| s.level < level)
                .map(|s| s.id);

            let title = line
                .trim_start_matches("### ")
                .trim_start_matches("## ")
                .trim_start_matches("# ")
                .trim();

            current_section = Some(Section {
                id: section_id_counter,
                doc_name: String::new(),
                level,
                title: title.to_string(),
                start_line: line_num,
                end_line: 0,
                parent_id,
                child_count: 0,
                content_hash: String::new(),
                is_confidence_section: false,
            });
            section_id_counter += 1;
        }

        // Accumulate content hash for current section
        if let Some(ref mut s) = current_section {
            let mut h: u64 = 0;
            for byte in line.bytes() {
                h = h.wrapping_add(byte as u64).wrapping_mul(31);
            }
            s.content_hash = format!("{:x}", h);
        }
    }

    // Close the last section
    if let Some(mut s) = current_section.take() {
        s.end_line = lines.len();
        sections.push(s);
    }

    sections
}

/// Extract tables from Markdown content
pub fn extract_tables(content: &str) -> Vec<Table> {
    let lines: Vec<&str> = content.lines().collect();
    let mut tables = Vec::new();
    let mut in_table = false;
    let mut current_table: Option<Table> = None;
    let _row_count = 0;
    let mut table_id_counter = 1i64;

    for (i, line) in lines.iter().enumerate() {
        let line_num = i + 1;

        // Detect table start (line with | characters and at least 2 columns)
        if !in_table && line.contains('|') && line.split('|').count() >= 3 {
            // Check if it's a separator line (---)
            if is_separator_line(line) {
                continue;
            }
            // Check if previous line is a header-like line
            if i > 0 {
                let prev_line = lines[i - 1];
                if prev_line.starts_with("# ") || prev_line.starts_with("## ") {
                    in_table = true;
                    current_table = Some(Table {
                        id: table_id_counter,
                        doc_name: String::new(),
                        section_id: 0,
                        start_line: line_num,
                        end_line: 0,
                        headers: extract_header_from_line(line),
                        row_count: 0,
                        content_hash: String::new(),
                    });
                    table_id_counter += 1;
                }
            }
        } else if in_table && line.contains('|') {
            if let Some(ref mut t) = current_table {
                let row = extract_row_from_line(line);
                t.row_count += 1;
                let mut h: u64 = 0;
                for byte in row.iter().flat_map(|s| s.bytes()) {
                    h = h.wrapping_add(byte as u64).wrapping_mul(31);
                }
                t.content_hash = format!("{:x}", h);
            }
        } else if in_table && !line.contains('|') {
            // End of table
            if let Some(mut t) = current_table.take() {
                t.end_line = line_num - 1;
                tables.push(t);
            }
            in_table = false;
        }
    }

    // Close any open table
    if in_table && let Some(mut t) = current_table.take() {
        t.end_line = lines.len();
        tables.push(t);
    }

    tables
}

/// Extract code blocks from Markdown content
pub fn extract_code_blocks(content: &str) -> Vec<CodeBlock> {
    let lines: Vec<&str> = content.lines().collect();
    let mut code_blocks = Vec::new();
    let mut in_block = false;
    let mut current_block: Option<CodeBlock> = None;
    let mut block_content = String::new();
    let mut block_id_counter = 1i64;

    for (i, line) in lines.iter().enumerate() {
        let line_num = i + 1;

        if !in_block && line.starts_with("```") {
            in_block = true;
            let lang = line.trim_start_matches("```").trim();
            current_block = Some(CodeBlock {
                id: block_id_counter,
                doc_name: String::new(),
                section_id: 0,
                start_line: line_num,
                end_line: 0,
                language: lang.to_string(),
                content_hash: String::new(),
                line_count: 0,
            });
            block_id_counter += 1;
            block_content.clear();
        } else if in_block && line.starts_with("```") {
            // End of block
            if let Some(ref mut b) = current_block {
                b.end_line = line_num;
                b.line_count = block_content.lines().count();
                let mut h: u64 = 0;
                for byte in block_content.bytes() {
                    h = h.wrapping_add(byte as u64).wrapping_mul(31);
                }
                b.content_hash = format!("{:x}", h);
                code_blocks.push(b.clone());
            }
            in_block = false;
            current_block = None;
            block_content.clear();
        } else if in_block {
            block_content.push_str(line);
            block_content.push('\n');
        }
    }

    code_blocks
}

/// Extract confidence assessment from Markdown content
pub fn extract_confidence(content: &str) -> Option<ConfidenceAssessment> {
    let lines: Vec<&str> = content.lines().collect();
    let mut in_confidence = false;
    let mut confidence: Option<ConfidenceAssessment> = None;
    let mut current_key: Option<String> = None;
    let mut current_value_lines = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        let _line_num = i + 1;

        // Detect confidence section (typically "## 8. Confidence Assessment")
        if line.starts_with("## 8. Confidence Assessment") {
            in_confidence = true;
            confidence = Some(ConfidenceAssessment {
                doc_name: String::new(),
                section_id: 0,
                what_captured: String::new(),
                what_missed: String::new(),
                assumptions: Vec::new(),
                blind_spots: Vec::new(),
                stale_after: String::new(),
                captured_at: Utc::now(),
            });
            continue;
        }

        if in_confidence {
            // Detect subsections
            if line.starts_with("### 8.1 What This Review Captures") {
                current_key = Some("what_captured".to_string());
                continue;
            }
            if line.starts_with("### 8.2 What This Review Might Have Missed") {
                current_key = Some("what_missed".to_string());
                continue;
            }
            if line.starts_with("### 8.3 Assumptions") {
                current_key = Some("assumptions".to_string());
                continue;
            }
            if line.starts_with("### 8.4 Blind Spots") {
                current_key = Some("blind_spots".to_string());
                continue;
            }
            if line.starts_with("### 8.5 Stale After") {
                current_key = Some("stale_after".to_string());
                continue;
            }

            // End of confidence section
            if line.starts_with("## ") && !line.starts_with("## 8.") {
                if let Some(ref mut c) = confidence {
                    match current_key.as_ref() {
                        Some(key) if key == "what_captured" => {
                            c.what_captured = current_value_lines.join("\n")
                        }
                        Some(key) if key == "what_missed" => {
                            c.what_missed = current_value_lines.join("\n")
                        }
                        Some(key) if key == "assumptions" => {
                            c.assumptions = current_value_lines.clone()
                        }
                        Some(key) if key == "blind_spots" => {
                            c.blind_spots = current_value_lines.clone()
                        }
                        Some(key) if key == "stale_after" => {
                            c.stale_after = current_value_lines.join("\n")
                        }
                        _ => {}
                    }
                }
                in_confidence = false;
                current_key = None;
                current_value_lines.clear();
                continue;
            }

            // Accumulate content for current key
            #[allow(clippy::collapsible_if)]
            if let Some(ref mut c) = confidence {
                if let Some(ref key) = current_key {
                    match key.as_str() {
                        "what_captured" => c.what_captured.push_str(line),
                        "what_missed" => c.what_missed.push_str(line),
                        "assumptions" => c.assumptions.push(line.trim().to_string()),
                        "blind_spots" => c.blind_spots.push(line.trim().to_string()),
                        "stale_after" => c.stale_after.push_str(line),
                        _ => {}
                    }
                    current_value_lines.push(line.to_string());
                }
            }
        }
    }

    // Close any open key
    #[allow(clippy::collapsible_if)]
    if in_confidence {
        if let Some(ref mut c) = confidence {
            match current_key.as_ref() {
                Some(key) if key == "what_captured" => {
                    c.what_captured = current_value_lines.join("\n")
                }
                Some(key) if key == "what_missed" => c.what_missed = current_value_lines.join("\n"),
                Some(key) if key == "assumptions" => c.assumptions = current_value_lines.clone(),
                Some(key) if key == "blind_spots" => c.blind_spots = current_value_lines.clone(),
                Some(key) if key == "stale_after" => c.stale_after = current_value_lines.join("\n"),
                _ => {}
            }
        }
    }

    confidence
}

/// Helper functions for table extraction.
fn is_separator_line(line: &str) -> bool {
    let parts: Vec<&str> = line.split('|').collect();
    parts.iter().all(|p| {
        let trimmed = p.trim();
        trimmed.is_empty() || trimmed.starts_with('-') || trimmed == "---"
    })
}

fn extract_header_from_line(line: &str) -> Vec<String> {
    line.split('|').map(|p| p.trim().to_string()).collect()
}

fn extract_row_from_line(line: &str) -> Vec<String> {
    line.split('|').map(|p| p.trim().to_string()).collect()
}
