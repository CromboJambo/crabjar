//! Database migration to add provenance tracking fields.

use rusqlite::Connection;

/// Migrate existing decision database to include provenance fields.
pub fn migrate_for_provenance(conn: &Connection) -> Result<(), Box<dyn std::error::Error>> {
    // Add criteria_version field
    conn.execute_batch(
        "ALTER TABLE decisions ADD COLUMN criteria_version TEXT DEFAULT '';
         ALTER TABLE decisions ADD COLUMN decision_inputs TEXT;
         ALTER TABLE decisions ADD COLUMN parent_decision_id INTEGER REFERENCES decisions(id);",
    )?;

    Ok(())
}
