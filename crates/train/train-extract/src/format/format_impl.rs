use crate::extract_types::{Annotation, Chunk, KnowledgeEntry, LogEvent};

/// Format a knowledge entry as instruction/response pair.
pub fn format_knowledge_entry(entry: &KnowledgeEntry) -> Vec<super::Sample> {
    use super::Sample;
    let mut samples = Vec::new();

    let tags_str = if entry.tags.is_empty() {
        String::new()
    } else {
        format!(" ({})", entry.tags.join(", "))
    };

    let kind_label = match entry.kind {
        agent_context::KnowledgeKind::Instruction => "instruction",
        agent_context::KnowledgeKind::Pattern => "pattern",
        agent_context::KnowledgeKind::Example => "example",
        agent_context::KnowledgeKind::Context => "context",
        agent_context::KnowledgeKind::Rule => "rule",
        agent_context::KnowledgeKind::Procedure => "procedure",
    };

    let instruction = format!(
        "What is the {kind_label}{tags_str}?\n\n{}",
        entry.content.lines().next().unwrap_or("")
    );

    let response = entry
        .content
        .lines()
        .skip_while(|l| l.is_empty())
        .take_while(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    let provenance = format!("knowledge:{}", entry.id);

    samples.push(Sample {
        instruction,
        response: if response.is_empty() {
            entry.content.clone()
        } else {
            response
        },
        source: super::SampleSource::KnowledgeEntry {
            entry_id: entry.id,
            kind: format!("{:?}", entry.kind),
        },
        weight: entry.weight,
        tags: entry.tags.clone(),
        provenance_id: provenance,
        created_at: entry.created_at,
    });

    samples
}

/// Format a mirror-log event as instruction/response pair.
pub fn format_log_event(event: &LogEvent) -> Vec<super::Sample> {
    use super::Sample;
    let content = event.content.trim();
    if content.is_empty() {
        return Vec::new();
    }

    let first_line = content.lines().next().unwrap_or(content);
    let instruction = format!(
        "What happened in the {source} event?\n\n{first_line}",
        source = event.source
    );

    let response = content
        .lines()
        .skip_while(|l| l.is_empty())
        .take_while(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    vec![Sample {
        instruction,
        response: if response.is_empty() {
            content.to_string()
        } else {
            response
        },
        source: super::SampleSource::LogEvent {
            event_id: event.id.clone(),
            source: event.source.clone(),
        },
        weight: 0.5,
        tags: vec![event.source.clone()],
        provenance_id: format!("event:{}", event.id),
        created_at: event.timestamp,
    }]
}

/// Format a chunk as instruction/response pair.
pub fn format_chunk(chunk: &Chunk) -> Vec<super::Sample> {
    use super::Sample;
    let content = chunk.content.trim();
    if content.is_empty() {
        return Vec::new();
    }

    let instruction = format!(
        "What is the content of chunk {idx}?\n\n{first}",
        idx = chunk.chunk_index + 1,
        first = content.lines().next().unwrap_or(content)
    );

    let response = content
        .lines()
        .skip_while(|l| l.is_empty())
        .take_while(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    vec![Sample {
        instruction,
        response: if response.is_empty() {
            content.to_string()
        } else {
            response
        },
        source: super::SampleSource::Chunk {
            chunk_id: chunk.id.clone(),
            event_id: chunk.event_id.clone(),
        },
        weight: 0.3,
        tags: vec!["chunk".to_string()],
        provenance_id: format!("chunk:{}", chunk.id),
        created_at: 0,
    }]
}

/// Format a resolved annotation as instruction/response pair.
pub fn format_annotation(annotation: &Annotation) -> Vec<super::Sample> {
    use super::Sample;
    let mut samples = Vec::new();

    if annotation.status != "resolved" {
        return samples;
    }

    let reason = match &annotation.resolution_reason {
        Some(r) => r.clone(),
        None => return samples,
    };

    let instruction = format!(
        "What decision was made about the {doc} annotation ({kind})?\n\n{message}",
        doc = annotation.doc_name,
        kind = annotation.kind,
        message = annotation.message
    );

    samples.push(Sample {
        instruction,
        response: reason,
        source: super::SampleSource::Annotation {
            doc_name: annotation.doc_name.clone(),
            annotation_id: annotation.id.clone(),
            annotation_kind: annotation.kind.clone(),
        },
        weight: annotation.confidence,
        tags: vec!["annotation".to_string(), annotation.doc_name.clone()],
        provenance_id: format!("annotation:{}:{}", annotation.doc_name, annotation.id),
        created_at: annotation.created_at,
    });

    samples
}
