//! HabitatStore — CRUD + snapshot reads for the spatial habitat.
//!
//! Mirrors the `state_docs::StateDocQuerier` pattern: a thin struct over a
//! `rusqlite::Connection` with structured reads. The store owns the spatial
//! model only — it never couples to the physical environment (HA) and never
//! auto-corrects divergence (ADR-003 decision 4).
use crate::error::Result;
use crate::habitat::models::{
    EntityKind, HabitatArea, HabitatDivergence, HabitatEntity, HabitatSnapshot,
};
use chrono::Utc;
use rusqlite::Connection;

/// CRUD + snapshot interface for the spatial habitat.
pub struct HabitatStore {
    conn: Connection,
}

impl HabitatStore {
    /// Open a store on an existing (already migrated) connection.
    pub fn new(conn: Connection) -> Self {
        Self { conn }
    }

    /// Open a store on a database path, applying migrations.
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        crate::habitat::schema::migrate(&conn)?;
        Ok(Self { conn })
    }

    /// Create an area with coarse grid geometry.
    ///
    /// Idempotent on name: returns the existing id if the area already exists.
    pub fn upsert_area(&self, name: &str, grid_w: i64, grid_h: i64) -> Result<i64> {
        if let Some(id) = self.area_id_by_name(name)? {
            return Ok(id);
        }
        self.conn.execute(
            "INSERT INTO habitat_areas (name, grid_w, grid_h) VALUES (?1, ?2, ?3)",
            rusqlite::params![name, grid_w, grid_h],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Id of an area by name, if it exists.
    pub fn area_id_by_name(&self, name: &str) -> Result<Option<i64>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM habitat_areas WHERE name = ?1")?;
        let rows = stmt
            .query_map(rusqlite::params![name], |row| row.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows.into_iter().next())
    }

    /// Insert or update a positioned entity.
    ///
    /// Upserts on the stable entity id (e.g. a guard pending-entry id or an
    /// agent id). Position is validated against the area's grid.
    pub fn upsert_entity(&self, entity: &HabitatEntity) -> Result<()> {
        let area = self
            .conn
            .query_row(
                "SELECT grid_w, grid_h FROM habitat_areas WHERE id = ?1",
                rusqlite::params![entity.area_id],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
            )
            .map_err(|e| {
                if e == rusqlite::Error::QueryReturnedNoRows {
                    crate::error::Error::NotFound(format!("area {} not found", entity.area_id))
                } else {
                    e.into()
                }
            })?;
        if entity.x < 0 || entity.x >= area.0 || entity.y < 0 || entity.y >= area.1 {
            return Err(crate::error::Error::Config(format!(
                "entity {} at ({}, {}) outside area {} grid {}x{}",
                entity.id, entity.x, entity.y, entity.area_id, area.0, area.1
            )));
        }
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO habitat_entities (id, area_id, kind, state, label, x, y, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
             ON CONFLICT(id) DO UPDATE SET
                area_id = excluded.area_id,
                kind = excluded.kind,
                state = excluded.state,
                label = excluded.label,
                x = excluded.x,
                y = excluded.y,
                updated_at = excluded.updated_at",
            rusqlite::params![
                entity.id,
                entity.area_id,
                entity.kind.as_str(),
                entity.state,
                entity.label,
                entity.x,
                entity.y,
                now
            ],
        )?;
        Ok(())
    }

    /// Remove an entity from the model (state was resolved, archived, or consumed).
    pub fn remove_entity(&self, id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM habitat_entities WHERE id = ?1",
            rusqlite::params![id],
        )?;
        Ok(())
    }

    /// Record a divergence between the physical environment and the model.
    ///
    /// Exposed, never auto-corrected: recording is the whole operation.
    pub fn record_divergence(&self, area_id: i64, description: &str) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO habitat_divergences (area_id, description) VALUES (?1, ?2)",
            rusqlite::params![area_id, description],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Mark a divergence resolved (the human decided which side is authoritative).
    pub fn resolve_divergence(&self, id: i64) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE habitat_divergences SET status = 'resolved', resolved_at = ?2 WHERE id = ?1",
            rusqlite::params![id, now],
        )?;
        Ok(())
    }

    /// All areas in the model.
    pub fn areas(&self) -> Result<Vec<HabitatArea>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, grid_w, grid_h FROM habitat_areas ORDER BY id")?;
        let rows = stmt
            .query_map([], |row| {
                Ok(HabitatArea {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    grid_w: row.get(2)?,
                    grid_h: row.get(3)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// All positioned entities.
    pub fn entities(&self) -> Result<Vec<HabitatEntity>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, area_id, kind, state, label, x, y, created_at, updated_at
             FROM habitat_entities ORDER BY area_id, x, y",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        rows.into_iter()
            .map(
                |(id, area_id, kind_str, state, label, x, y, created_at, updated_at)| {
                    let kind = EntityKind::parse(&kind_str).ok_or_else(|| {
                        crate::error::Error::NotFound(format!("unknown entity kind {kind_str}"))
                    })?;
                    Ok(HabitatEntity {
                        id,
                        area_id,
                        kind,
                        state,
                        label,
                        x,
                        y,
                        created_at,
                        updated_at,
                    })
                },
            )
            .collect::<Result<Vec<_>>>()
    }

    /// Divergences in an area (open and resolved).
    pub fn divergences_in_area(&self, area_id: i64) -> Result<Vec<HabitatDivergence>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, area_id, description, status, created_at, resolved_at
             FROM habitat_divergences WHERE area_id = ?1 ORDER BY id",
        )?;
        let rows = stmt
            .query_map(rusqlite::params![area_id], |row| {
                Ok(HabitatDivergence {
                    id: row.get(0)?,
                    area_id: row.get(1)?,
                    description: row.get(2)?,
                    status: row.get(3)?,
                    created_at: row.get(4)?,
                    resolved_at: row.get(5)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Full render-ready snapshot: areas, entities, and divergences.
    pub fn snapshot(&self) -> Result<HabitatSnapshot> {
        let areas = self.areas()?;
        let entities = self.entities()?;
        let mut divergences = Vec::new();
        for area in &areas {
            divergences.extend(self.divergences_in_area(area.id)?);
        }
        Ok(HabitatSnapshot {
            areas,
            entities,
            divergences,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::habitat::schema::migrate;

    fn store() -> HabitatStore {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        HabitatStore::new(conn)
    }

    fn entity(id: &str, area_id: i64, kind: EntityKind, x: i64, y: i64) -> HabitatEntity {
        HabitatEntity {
            id: id.to_string(),
            area_id,
            kind,
            state: "idle".to_string(),
            label: id.to_string(),
            x,
            y,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    #[test]
    fn upsert_area_is_idempotent_on_name() {
        let store = store();
        let a = store.upsert_area("desk", 16, 4).unwrap();
        let b = store.upsert_area("desk", 8, 2).unwrap();
        assert_eq!(a, b);
        assert_eq!(store.areas().unwrap().len(), 1);
    }

    #[test]
    fn upsert_entity_roundtrips_and_updates() {
        let store = store();
        let area = store.upsert_area("desk", 16, 4).unwrap();
        store
            .upsert_entity(&entity("agent-1", area, EntityKind::Agent, 2, 1))
            .unwrap();
        let mut moved = entity("agent-1", area, EntityKind::Agent, 5, 3);
        moved.state = "working".to_string();
        store.upsert_entity(&moved).unwrap();

        let entities = store.entities().unwrap();
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].x, 5);
        assert_eq!(entities[0].y, 3);
        assert_eq!(entities[0].state, "working");
        assert_eq!(entities[0].kind, EntityKind::Agent);
    }

    #[test]
    fn upsert_entity_rejects_out_of_bounds_position() {
        let store = store();
        let area = store.upsert_area("desk", 16, 4).unwrap();
        let err = store
            .upsert_entity(&entity("agent-1", area, EntityKind::Agent, 16, 0))
            .unwrap_err();
        assert!(matches!(err, crate::error::Error::Config(_)));
    }

    #[test]
    fn upsert_entity_rejects_unknown_area() {
        let store = store();
        let err = store
            .upsert_entity(&entity("agent-1", 99, EntityKind::Agent, 0, 0))
            .unwrap_err();
        assert!(matches!(err, crate::error::Error::NotFound(_)));
    }

    #[test]
    fn remove_entity_clears_clutter() {
        let store = store();
        let area = store.upsert_area("desk", 16, 4).unwrap();
        store
            .upsert_entity(&entity("agent-1", area, EntityKind::Agent, 0, 0))
            .unwrap();
        assert_eq!(store.snapshot().unwrap().clutter(), 1);
        store.remove_entity("agent-1").unwrap();
        assert_eq!(store.snapshot().unwrap().clutter(), 0);
    }

    #[test]
    fn divergence_lifecycle_open_to_resolved() {
        let store = store();
        let area = store.upsert_area("desk", 16, 4).unwrap();
        let d = store
            .record_divergence(
                area,
                "presence sensor says desk is empty; model has 3 agents",
            )
            .unwrap();
        let snap = store.snapshot().unwrap();
        assert_eq!(snap.open_divergences(), 1);
        assert!(snap.divergences[0].is_open());

        store.resolve_divergence(d).unwrap();
        let snap = store.snapshot().unwrap();
        assert_eq!(snap.open_divergences(), 0);
        assert!(!snap.divergences[0].is_open());
        assert!(snap.divergences[0].resolved_at.is_some());
    }

    #[test]
    fn snapshot_groups_entities_by_area() {
        let store = store();
        let desk = store.upsert_area("desk", 16, 4).unwrap();
        let kitchen = store.upsert_area("kitchen", 8, 4).unwrap();
        store
            .upsert_entity(&entity("a1", desk, EntityKind::Agent, 0, 0))
            .unwrap();
        store
            .upsert_entity(&entity("g1", desk, EntityKind::PendingGuardAction, 1, 0))
            .unwrap();
        store
            .upsert_entity(&entity("a2", kitchen, EntityKind::SuspendedRuntime, 0, 0))
            .unwrap();

        let snap = store.snapshot().unwrap();
        assert_eq!(snap.clutter(), 3);
        assert_eq!(snap.entities_in(desk).len(), 2);
        assert_eq!(snap.entities_in(kitchen).len(), 1);
    }
}
