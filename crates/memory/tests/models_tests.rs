use agent_context::state_docs::models::{
    Annotation, CodeBlock, ConfidenceAssessment, DocMetadata, Section, Table,
};

#[test]
fn doc_metadata_serializes_and_deserializes() {
    let meta = DocMetadata {
        doc_name: "project-map".to_string(),
        display_name: "Project Map".to_string(),
        description: "Overview".to_string(),
        path: "state-docs/project-map.md".to_string(),
        last_modified: chrono::Utc::now(),
        line_count: 42,
        section_count: 5,
        table_count: 1,
        code_block_count: 2,
        annotation_count: 3,
        open_annotation_count: 1,
        checksum: "abc123".to_string(),
    };

    let json = serde_json::to_value(&meta).unwrap();
    assert_eq!(json["doc_name"], "project-map");
    assert_eq!(json["line_count"], 42);
    assert_eq!(json["table_count"], 1);
    assert_eq!(json["checksum"], "abc123");

    let back: DocMetadata = serde_json::from_value(json).unwrap();
    assert_eq!(back.doc_name, "project-map");
    assert_eq!(back.line_count, 42);
}

#[test]
fn confidence_assessment_serializes_default() {
    let conf = ConfidenceAssessment::default();
    let json = serde_json::to_value(&conf).unwrap();
    assert_eq!(json["assumptions"], serde_json::json!([]));
    assert_eq!(json["blind_spots"], serde_json::json!([]));
    assert_eq!(json["what_captured"], "");
    assert_eq!(json["stale_after"], "");
}

#[test]
fn section_serializes_with_all_fields() {
    let section = Section {
        id: 1,
        doc_name: "test".to_string(),
        level: 2,
        title: "Architecture".to_string(),
        start_line: 10,
        end_line: 30,
        parent_id: Some(0),
        child_count: 3,
        content_hash: "deadbeef".to_string(),
        is_confidence_section: false,
    };

    let json = serde_json::to_value(&section).unwrap();
    assert_eq!(json["level"], 2);
    assert_eq!(json["start_line"], 10);
    assert_eq!(json["end_line"], 30);
    assert!(json["parent_id"].is_number());
}

#[test]
fn annotation_serializes_with_status() {
    let ann = Annotation {
        id: 42,
        doc_name: "test".to_string(),
        section_id: Some(1),
        line: 15,
        kind: "note".to_string(),
        message: "check this".to_string(),
        author: "agent".to_string(),
        status: "open".to_string(),
        created_at: chrono::Utc::now(),
    };

    let json = serde_json::to_value(&ann).unwrap();
    assert_eq!(json["status"], "open");
    assert_eq!(json["kind"], "note");
    assert_eq!(json["line"], 15);
}

#[test]
fn code_block_serializes_with_language() {
    let block = CodeBlock {
        id: 1,
        doc_name: "test".to_string(),
        section_id: 2,
        start_line: 20,
        end_line: 25,
        language: "rust".to_string(),
        content_hash: "cafebabe".to_string(),
        line_count: 6,
    };

    let json = serde_json::to_value(&block).unwrap();
    assert_eq!(json["language"], "rust");
    assert_eq!(json["line_count"], 6);
}

#[test]
fn table_serializes_with_headers() {
    let table = Table {
        id: 1,
        doc_name: "test".to_string(),
        section_id: 0,
        start_line: 12,
        end_line: 16,
        headers: vec![
            "Crate".to_string(),
            "Path".to_string(),
            "Purpose".to_string(),
        ],
        row_count: 3,
        content_hash: "1234abcd".to_string(),
    };

    let json = serde_json::to_value(&table).unwrap();
    assert_eq!(json["row_count"], 3);
    assert_eq!(json["headers"][0], "Crate");
    assert_eq!(json["headers"][2], "Purpose");
}
