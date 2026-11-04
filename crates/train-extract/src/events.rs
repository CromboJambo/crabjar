use crate::error::{TrainExtractError, TrainExtractResult};
use crate::extract_types::LogEvent;
use rusqlite::Connection;

/// Extract events from the mirror-log database.
pub fn extract_events(
    conn: Option<&Connection>,
    since: i64,
    max_entries: usize,
) -> TrainExtractResult<Vec<LogEvent>> {
    let Some(conn) = conn else {
        return Ok(Vec::new());
    };

    let mut stmt = conn.prepare(
        "SELECT id, content, source, meta, timestamp
         FROM events
         WHERE (?1 = 0 OR timestamp >= ?1)
         ORDER BY timestamp DESC
         LIMIT ?2",
    )?;

    let rows = stmt.query_map([since, max_entries as i64], |row| {
        Ok(LogEvent {
            id: row.get(0)?,
            content: row.get(1)?,
            source: row.get(2)?,
            meta: row.get(3)?,
            timestamp: row.get(4)?,
        })
    })?;

    let events: Vec<LogEvent> = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| TrainExtractError::Export(format!("failed to read events: {e}")))?;

    tracing::debug!(
        events_extracted = events.len(),
        "Extracted mirror-log events"
    );

    Ok(events)
}
