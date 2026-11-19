use crate::error::{Error, Result};
use crate::waypoints::models::*;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde_json;
use std::sync::Mutex;

/// Trait for navigating the waypoint hierarchy.
///
/// Workers enter at the root and drill down as needed. Each level provides
/// enough context to decide whether to continue deeper or look elsewhere.
pub trait WaypointNavigator: Send + Sync {
    /// Create a new waypoint in the store.
    fn create(&self, waypoint: &Waypoint) -> Result<Waypoint>;

    /// Update an existing waypoint.
    fn update(&self, waypoint: &Waypoint) -> Result<Waypoint>;

    /// Get a waypoint by ID.
    fn get(&self, id: &WaypointId) -> Result<Waypoint>;

    /// Set the root waypoint for an environment.
    fn set_root(&self, env: &EnvironmentId, root_id: &WaypointId) -> Result<()>;

    /// Get the root waypoint for an environment.
    fn root(&self, env: &EnvironmentId) -> Result<Waypoint>;

    /// Get child waypoints of a given waypoint (by parent_id).
    fn children(&self, id: &WaypointId) -> Result<Vec<Waypoint>>;

    /// Get destinations from a waypoint (explicit links).
    fn destinations(&self, id: &WaypointId) -> Result<Vec<Waypoint>>;

    /// Traverse from current to destination with depth limit.
    fn traverse(
        &self,
        worker_id: &str,
        start: &WaypointId,
        max_depth: u32,
    ) -> Result<TraversalPath>;

    /// Find waypoints by category within a scope (subtree of scope waypoint).
    fn find_by_category(
        &self,
        scope: &WaypointId,
        category: WaypointCategory,
    ) -> Result<Vec<Waypoint>>;

    /// Get the lineage (ancestry chain via predecessor links) of a waypoint.
    fn lineage(&self, id: &WaypointId) -> Result<Vec<Waypoint>>;

    /// Get the full ancestry chain (via parent_id) from a waypoint to root.
    fn ancestry(&self, id: &WaypointId) -> Result<Vec<Waypoint>>;

    /// Verify freshness of a waypoint against a threshold.
    fn check_freshness(&self, id: &WaypointId, threshold_hours: i64) -> Result<FreshnessCheck>;

    /// Refresh the freshness timestamp of a waypoint (after re-verification).
    fn refresh(&self, id: &WaypointId) -> Result<Waypoint>;

    /// Delete a waypoint and all its descendants.
    fn delete_subtree(&self, id: &WaypointId) -> Result<usize>;
}

/// SQLite-backed implementation of WaypointNavigator.
pub struct SqliteWaypointNavigator {
    conn: Mutex<Connection>,
}

impl SqliteWaypointNavigator {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Mutex::new(conn),
        }
    }

    fn lock_conn(&self) -> Result<std::sync::MutexGuard<'_, Connection>> {
        self.conn
            .lock()
            .map_err(|e| Error::Internal(format!("waypoint navigator poisoned: {e}")))
    }

    fn row_to_waypoint(row: &rusqlite::Row) -> rusqlite::Result<Waypoint> {
        let id: String = row.get(0)?;
        let parent_id: Option<String> = row.get(1)?;
        let environment: String = row.get(2)?;
        let category_str: String = row.get(3)?;
        let description: String = row.get(4)?;
        let destinations_json: String = row.get(5)?;
        let references_json: String = row.get(6)?;
        let relevance: f32 = row.get(7)?;
        let freshness_ts: i64 = row.get(8)?;
        let predecessor_id: Option<String> = row.get(9)?;
        let transformation_id: Option<String> = row.get(10)?;
        let recoverability_str: String = row.get(11)?;
        let created_at_ts: i64 = row.get(12)?;
        let updated_at_ts: i64 = row.get(13)?;

        let category = WaypointCategory::parse(&category_str)
            .ok_or(rusqlite::Error::InvalidParameterName("category".into()))?;
        let recoverability =
            Recoverability::parse(&recoverability_str).unwrap_or(Recoverability::Unknown);

        Ok(Waypoint {
            id: WaypointId(id),
            parent_id: parent_id.map(WaypointId),
            environment: EnvironmentId(environment),
            category,
            description,
            destinations: serde_json::from_str(&destinations_json).unwrap_or_default(),
            references: serde_json::from_str(&references_json).unwrap_or_default(),
            relevance,
            freshness: DateTime::from_timestamp(freshness_ts, 0).unwrap_or_else(Utc::now),
            predecessor: predecessor_id.map(WaypointId),
            transformation: transformation_id.map(FoldRef),
            recoverability,
            created_at: DateTime::from_timestamp(created_at_ts, 0).unwrap_or_else(Utc::now),
            updated_at: DateTime::from_timestamp(updated_at_ts, 0).unwrap_or_else(Utc::now),
        })
    }

    const WAYPOINT_COLUMNS: &'static str = "
        id, parent_id, environment, category, description, destinations, [references],
        relevance, freshness, predecessor_id, transformation_id, recoverability,
        created_at, updated_at
    ";
}

impl WaypointNavigator for SqliteWaypointNavigator {
    fn create(&self, wp: &Waypoint) -> Result<Waypoint> {
        let now = Utc::now().timestamp();
        let destinations_json = serde_json::to_string(&wp.destinations)?;
        let references_json = serde_json::to_string(&wp.references)?;
        let recoverability_str = wp.recoverability.to_string();

        self.lock_conn()?.execute(
            "INSERT INTO waypoints (
                id, parent_id, environment, category, description, destinations, [references],
                relevance, freshness, predecessor_id, transformation_id, recoverability,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                wp.id.as_str(),
                wp.parent_id.as_ref().map(|p| p.as_str()),
                wp.environment.as_str(),
                wp.category.as_str(),
                wp.description,
                destinations_json,
                references_json,
                wp.relevance,
                wp.freshness.timestamp(),
                wp.predecessor.as_ref().map(|p| p.as_str()),
                wp.transformation.as_ref().map(|t| t.as_str()),
                recoverability_str,
                now,
                now,
            ],
        )?;

        self.get(&wp.id)
    }

    fn update(&self, wp: &Waypoint) -> Result<Waypoint> {
        let now = Utc::now().timestamp();
        let destinations_json = serde_json::to_string(&wp.destinations)?;
        let references_json = serde_json::to_string(&wp.references)?;
        let recoverability_str = wp.recoverability.to_string();

        let rows = self.lock_conn()?.execute(
            "UPDATE waypoints SET
                parent_id = ?, environment = ?, category = ?, description = ?,
                destinations = ?, [references] = ?, relevance = ?, freshness = ?,
                predecessor_id = ?, transformation_id = ?, recoverability = ?, updated_at = ?
             WHERE id = ?",
            params![
                wp.parent_id.as_ref().map(|p| p.as_str()),
                wp.environment.as_str(),
                wp.category.as_str(),
                wp.description,
                destinations_json,
                references_json,
                wp.relevance,
                wp.freshness.timestamp(),
                wp.predecessor.as_ref().map(|p| p.as_str()),
                wp.transformation.as_ref().map(|t| t.as_str()),
                recoverability_str,
                now,
                wp.id.as_str(),
            ],
        )?;

        if rows == 0 {
            return Err(Error::NotFound(format!("waypoint {} not found", wp.id)));
        }

        self.get(&wp.id)
    }

    fn get(&self, id: &WaypointId) -> Result<Waypoint> {
        let sql = format!(
            "SELECT {} FROM waypoints WHERE id = ?",
            Self::WAYPOINT_COLUMNS
        );
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query([id.as_str()])?;
        match rows.next()? {
            Some(row) => Self::row_to_waypoint(row).map_err(Error::Database),
            None => Err(Error::NotFound(format!("waypoint {}", id))),
        }
    }

    fn set_root(&self, env: &EnvironmentId, root_id: &WaypointId) -> Result<()> {
        let now = Utc::now().timestamp();
        self.lock_conn()?.execute(
            "INSERT OR REPLACE INTO waypoint_roots (environment, root_id, created_at, updated_at)
             VALUES (?, ?, COALESCE((SELECT created_at FROM waypoint_roots WHERE environment = ?), ?), ?)",
            params![env.as_str(), root_id.as_str(), env.as_str(), now, now],
        )?;
        Ok(())
    }

    fn root(&self, env: &EnvironmentId) -> Result<Waypoint> {
        let root_id: String = self
            .lock_conn()?
            .query_row(
                "SELECT root_id FROM waypoint_roots WHERE environment = ?",
                [env.as_str()],
                |row| row.get(0),
            )
            .map_err(|_| Error::NotFound(format!("root waypoint for env {}", env)))?;
        self.get(&WaypointId(root_id))
    }

    fn children(&self, id: &WaypointId) -> Result<Vec<Waypoint>> {
        let sql = format!(
            "SELECT {} FROM waypoints WHERE parent_id = ? ORDER BY relevance DESC",
            Self::WAYPOINT_COLUMNS
        );
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([id.as_str()], Self::row_to_waypoint)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Error::Database)
    }

    fn destinations(&self, id: &WaypointId) -> Result<Vec<Waypoint>> {
        let wp = self.get(id)?;
        let mut results = Vec::new();
        for dest_id in &wp.destinations {
            if let Ok(dest) = self.get(dest_id) {
                results.push(dest);
            }
        }
        Ok(results)
    }

    fn traverse(
        &self,
        worker_id: &str,
        start: &WaypointId,
        max_depth: u32,
    ) -> Result<TraversalPath> {
        let mut path = Vec::new();
        let mut current_id = start.clone();
        let mut reached_target = false;

        for _ in 0..=max_depth {
            let wp = match self.get(&current_id) {
                Ok(wp) => wp,
                Err(Error::NotFound(_)) => break,
                Err(e) => return Err(e),
            };
            path.push(wp.clone());

            // If this is a task waypoint or has no destinations, we've reached our target
            if wp.category == WaypointCategory::Task || wp.destinations.is_empty() {
                reached_target = true;
                break;
            }

            // Follow first destination (simplest traversal strategy)
            match wp.destinations.first() {
                Some(next_id) => current_id = next_id.clone(),
                None => break,
            }
        }

        // Log the traversal for audit
        let end_id = path
            .last()
            .map(|w| w.id.clone())
            .unwrap_or_else(|| start.clone());
        let now = Utc::now().timestamp();
        self.lock_conn()?.execute(
            "INSERT INTO waypoint_traversals (worker_id, start_id, end_id, hops, depth_limit, timestamp)
             VALUES (?, ?, ?, ?, ?, ?)",
            params![
                worker_id,
                start.as_str(),
                end_id.as_str(),
                path.len().saturating_sub(1),
                Some(max_depth as i64),
                now,
            ],
        )?;

        Ok(TraversalPath {
            total_hops: path.len().saturating_sub(1),
            reached_target,
            waypoints: path,
        })
    }

    fn find_by_category(
        &self,
        scope: &WaypointId,
        category: WaypointCategory,
    ) -> Result<Vec<Waypoint>> {
        // Collect all descendant IDs of scope via BFS on parent_id
        let mut descendants = vec![scope.as_str().to_string()];
        let mut queue = vec![scope.as_str().to_string()];

        while let Some(current) = queue.pop() {
            let conn = self.lock_conn()?;
            let mut stmt = conn.prepare("SELECT id FROM waypoints WHERE parent_id = ?")?;
            let ids: Vec<String> = stmt
                .query_map([current], |row| row.get(0))?
                .filter_map(|r| r.ok())
                .collect();
            for id in ids {
                descendants.push(id.clone());
                queue.push(id);
            }
        }

        if descendants.is_empty() {
            return Ok(Vec::new());
        }

        // Query all waypoints in the subtree with matching category
        let placeholders = descendants
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "SELECT {} FROM waypoints WHERE (id IN ({}) OR parent_id IN ({})) AND category = ? ORDER BY relevance DESC",
            Self::WAYPOINT_COLUMNS,
            placeholders,
            placeholders
        );

        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        for s in &descendants {
            params.push(Box::new(s.clone()));
        }
        for s in &descendants {
            params.push(Box::new(s.clone()));
        }
        params.push(Box::new(category.as_str().to_string()));

        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(&sql)?;
        let refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|b| b.as_ref()).collect();
        let rows = stmt.query_map(refs.as_slice(), Self::row_to_waypoint)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Error::Database)
    }

    fn lineage(&self, id: &WaypointId) -> Result<Vec<Waypoint>> {
        let mut chain = Vec::new();
        let mut current_id = id.clone();

        while let Ok(wp) = self.get(&current_id) {
            chain.push(wp.clone());
            match wp.predecessor {
                Some(pred_id) => current_id = pred_id,
                None => break,
            }
        }

        // Reverse so oldest (source) is first
        chain.reverse();
        Ok(chain)
    }

    fn ancestry(&self, id: &WaypointId) -> Result<Vec<Waypoint>> {
        let mut chain = Vec::new();
        let mut current_id = id.clone();

        while let Ok(wp) = self.get(&current_id) {
            chain.push(wp.clone());
            match wp.parent_id {
                Some(parent_id) => current_id = parent_id,
                None => break,
            }
        }

        // Reverse so root is first
        chain.reverse();
        Ok(chain)
    }

    fn check_freshness(&self, id: &WaypointId, threshold_hours: i64) -> Result<FreshnessCheck> {
        let wp = self.get(id)?;
        let age_hours = Utc::now().signed_duration_since(wp.freshness).num_hours();
        let is_fresh = age_hours < threshold_hours;

        Ok(FreshnessCheck {
            waypoint_id: id.clone(),
            is_fresh,
            age_hours,
            threshold_hours,
            message: if is_fresh {
                format!("Waypoint is fresh ({}h old)", age_hours)
            } else {
                format!(
                    "Waypoint is stale ({}h old, threshold {}h) — re-verification recommended",
                    age_hours, threshold_hours
                )
            },
        })
    }

    fn refresh(&self, id: &WaypointId) -> Result<Waypoint> {
        let mut wp = self.get(id)?;
        wp.freshness = Utc::now();
        wp.updated_at = Utc::now();
        self.update(&wp)
    }

    fn delete_subtree(&self, id: &WaypointId) -> Result<usize> {
        // Collect all descendant IDs
        let mut to_delete = vec![id.as_str().to_string()];
        let mut queue = vec![id.as_str().to_string()];

        while let Some(current) = queue.pop() {
            let conn = self.lock_conn()?;
            let mut stmt = conn.prepare("SELECT id FROM waypoints WHERE parent_id = ?")?;
            let ids: Vec<String> = stmt
                .query_map([current], |row| row.get(0))?
                .filter_map(|r| r.ok())
                .collect();
            for child_id in ids {
                to_delete.push(child_id.clone());
                queue.push(child_id);
            }
        }

        // Delete from waypoints table
        let placeholders = to_delete.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let sql = format!("DELETE FROM waypoints WHERE id IN ({})", placeholders);
        let params: Vec<&dyn rusqlite::ToSql> = to_delete
            .iter()
            .map(|s| s as &dyn rusqlite::ToSql)
            .collect();

        let deleted = self.lock_conn()?.execute(&sql, params.as_slice())?;

        // Also clear from waypoint_roots if any were roots
        for env_id in to_delete.iter() {
            self.lock_conn()?
                .execute("DELETE FROM waypoint_roots WHERE root_id = ?", [env_id])?;
        }

        Ok(deleted)
    }
}

#[cfg(test)]
mod tests;
