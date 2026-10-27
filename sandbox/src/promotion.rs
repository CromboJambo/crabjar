//! Worker promotion lifecycle management.
//!
//! Workers start as ephemeral Podman containers and can be promoted through
//! increasing levels of persistence and access:
//!
//! 1. Task Worker (Podman container, ephemeral)
//! 2. Gig Worker (persistent container)
//! 3. Dinit Service (system-level service)
//! 4. Shared User (crombo/user level configs and shell env)
//! 5. Crabjar User (own configs and shell env for experimentation)

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::SandboxError;
use crate::schema::{init_db, SandboxRow};
use rusqlite::Connection;
use tracing::{debug, info};

/// Worker promotion tiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromotionTier {
    /// Ephemeral Podman container for individual tasks.
    TaskWorker = 1,
    /// Persistent Podman container across restarts.
    GigWorker = 2,
    /// System-level dinit service managed by the host.
    DinitService = 3,
    /// Shared user account with crombo/user level configs.
    SharedUser = 4,
    /// Dedicated crabjar user with own configs and shell env.
    CrabjarUser = 5,
}

impl PromotionTier {
    pub fn name(&self) -> &'static str {
        match self {
            PromotionTier::TaskWorker => "task_worker",
            PromotionTier::GigWorker => "gig_worker",
            PromotionTier::DinitService => "dinit_service",
            PromotionTier::SharedUser => "shared_user",
            PromotionTier::CrabjarUser => "crabjar_user",
        }
    }

    pub fn from_name(name: &str) -> Option<PromotionTier> {
        match name {
            "task_worker" => Some(PromotionTier::TaskWorker),
            "gig_worker" => Some(PromotionTier::GigWorker),
            "dinit_service" => Some(PromotionTier::DinitService),
            "shared_user" => Some(PromotionTier::SharedUser),
            "crabjar_user" => Some(PromotionTier::CrabjarUser),
            _ => None,
        }
    }
}

/// Worker metadata for promotion tracking.
#[derive(Debug, Clone)]
pub struct WorkerMetadata {
    pub agent_name: String,
    pub tier: PromotionTier,
    pub container_id: Option<String>,
    pub service_name: Option<String>,
    pub username: Option<String>,
    pub home_path: Option<PathBuf>,
    pub created_at: i64,
    pub promoted_at: Option<i64>,
}

/// Worker promotion manager.
pub struct PromotionManager {
    conn: Connection,
    base_dir: PathBuf,
}

impl PromotionManager {
    /// Create a new promotion manager.
    pub fn new(base_dir: PathBuf) -> Result<Self, SandboxError> {
        let db_path = base_dir.join("promotion.db");
        let conn = Connection::open(&db_path)?;
        init_db(&conn)?;
        Ok(Self { conn, base_dir })
    }

    /// Create a new task worker (tier 1).
    pub fn create_task_worker(&self, agent_name: &str) -> Result<WorkerMetadata, SandboxError> {
        let sandbox_id = self.conn.execute(
            "INSERT INTO agent_sandboxes (agent_name, isolation_type, created_at) VALUES (?, ?, ?)",
            rusqlite::params![
                agent_name,
                PromotionTier::TaskWorker.name(),
                chrono::Utc::now().timestamp()
            ],
        )?;

        let metadata = WorkerMetadata {
            agent_name: agent_name.to_string(),
            tier: PromotionTier::TaskWorker,
            container_id: None,
            service_name: None,
            username: None,
            home_path: None,
            created_at: chrono::Utc::now().timestamp(),
            promoted_at: None,
        };

        info!(agent_name = %agent_name, "Task worker created");
        Ok(metadata)
    }

    /// Promote a task worker to gig worker (persistent container).
    pub fn promote_to_gig_worker(&self, agent_name: &str) -> Result<WorkerMetadata, SandboxError> {
        let sandbox = self.query_sandbox(agent_name)?;
        if sandbox.tier != PromotionTier::TaskWorker {
            return Err(SandboxError::Internal(format!(
                "Worker {} is not at task worker tier",
                agent_name
            )));
        }

        // Create persistent container configuration
        let container_id = format!("gig-{}", agent_name);
        self.conn.execute(
            "UPDATE agent_sandboxes SET isolation_type = ?, metadata = ? WHERE agent_name = ?",
            rusqlite::params![
                PromotionTier::GigWorker.name(),
                serde_json::to_string(&serde_json::json!({"container_id": container_id}))?,
                agent_name
            ],
        )?;

        let metadata = WorkerMetadata {
            agent_name: agent_name.to_string(),
            tier: PromotionTier::GigWorker,
            container_id: Some(container_id),
            service_name: None,
            username: None,
            home_path: None,
            created_at: sandbox.created_at,
            promoted_at: Some(chrono::Utc::now().timestamp()),
        };

        info!(agent_name = %agent_name, "Promoted to gig worker");
        Ok(metadata)
    }

    /// Promote a gig worker to dinit service.
    pub fn promote_to_dinit_service(
        &self,
        agent_name: &str,
    ) -> Result<WorkerMetadata, SandboxError> {
        let sandbox = self.query_sandbox(agent_name)?;
        if sandbox.tier != PromotionTier::GigWorker {
            return Err(SandboxError::Internal(format!(
                "Worker {} is not at gig worker tier",
                agent_name
            )));
        }

        // Generate dinit service configuration
        let service_name = format!("crabjar-{}", agent_name);
        self.conn.execute(
            "UPDATE agent_sandboxes SET isolation_type = ?, metadata = ? WHERE agent_name = ?",
            rusqlite::params![
                PromotionTier::DinitService.name(),
                serde_json::to_string(&serde_json::json!({"service_name": service_name}))?,
                agent_name
            ],
        )?;

        let metadata = WorkerMetadata {
            agent_name: agent_name.to_string(),
            tier: PromotionTier::DinitService,
            container_id: sandbox.container_id,
            service_name: Some(service_name),
            username: None,
            home_path: None,
            created_at: sandbox.created_at,
            promoted_at: Some(chrono::Utc::now().timestamp()),
        };

        info!(agent_name = %agent_name, "Promoted to dinit service");
        Ok(metadata)
    }

    /// Promote a dinit service to shared user.
    pub fn promote_to_shared_user(&self, agent_name: &str) -> Result<WorkerMetadata, SandboxError> {
        let sandbox = self.query_sandbox(agent_name)?;
        if sandbox.tier != PromotionTier::DinitService {
            return Err(SandboxError::Internal(format!(
                "Worker {} is not at dinit service tier",
                agent_name
            )));
        }

        // Create shared user account with crombo/user level configs
        let username = format!("crabjar-{}", agent_name);
        let home_path = PathBuf::from(format!("/home/{}", username));
        self.conn.execute(
            "UPDATE agent_sandboxes SET isolation_type = ?, metadata = ? WHERE agent_name = ?",
            rusqlite::params![
                PromotionTier::SharedUser.name(),
                serde_json::to_string(
                    &serde_json::json!({"username": username, "home_path": home_path.to_string()})
                )?,
                agent_name
            ],
        )?;

        let metadata = WorkerMetadata {
            agent_name: agent_name.to_string(),
            tier: PromotionTier::SharedUser,
            container_id: sandbox.container_id,
            service_name: sandbox.service_name,
            username: Some(username),
            home_path: Some(home_path),
            created_at: sandbox.created_at,
            promoted_at: Some(chrono::Utc::now().timestamp()),
        };

        info!(agent_name = %agent_name, "Promoted to shared user");
        Ok(metadata)
    }

    /// Promote a shared user to crabjar user with own configs.
    pub fn promote_to_crabjar_user(
        &self,
        agent_name: &str,
    ) -> Result<WorkerMetadata, SandboxError> {
        let sandbox = self.query_sandbox(agent_name)?;
        if sandbox.tier != PromotionTier::SharedUser {
            return Err(SandboxError::Internal(format!(
                "Worker {} is not at shared user tier",
                agent_name
            )));
        }

        // Create dedicated crabjar user with own configs and shell env
        let username = format!("crabjar-{}", agent_name);
        let home_path = PathBuf::from(format!("/home/{}", username));
        self.conn.execute(
            "UPDATE agent_sandboxes SET isolation_type = ?, metadata = ? WHERE agent_name = ?",
            rusqlite::params![
                PromotionTier::CrabjarUser.name(),
                serde_json::to_string(
                    &serde_json::json!({"username": username, "home_path": home_path.to_string()})
                )?,
                agent_name
            ],
        )?;

        let metadata = WorkerMetadata {
            agent_name: agent_name.to_string(),
            tier: PromotionTier::CrabjarUser,
            container_id: sandbox.container_id,
            service_name: sandbox.service_name,
            username: Some(username),
            home_path: Some(home_path),
            created_at: sandbox.created_at,
            promoted_at: Some(chrono::Utc::now().timestamp()),
        };

        info!(agent_name = %agent_name, "Promoted to crabjar user");
        Ok(metadata)
    }

    /// Query worker metadata.
    fn query_sandbox(&self, agent_name: &str) -> Result<WorkerMetadata, SandboxError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, agent_name, isolation_type, home_path, shell_config, cache_dirs, network_egress, resource_limits, sudo_policy, mount_scopes, created_at, active FROM agent_sandboxes WHERE agent_name = ? AND active = 1 LIMIT 1"
        )?;

        let row = stmt.query_row(rusqlite::params![agent_name], |row| {
            Ok(SandboxRow {
                id: row.get(0)?,
                agent_name: row.get(1)?,
                isolation_type: row.get(2)?,
                home_path: row.get(3)?,
                shell_config: row.get(4)?,
                cache_dirs: row.get(5)?,
                network_egress: row.get(6)?,
                resource_limits: row.get(7)?,
                sudo_policy: row.get(8)?,
                mount_scopes: row.get(9)?,
                created_at: row.get(10)?,
                active: row.get(11)?,
            })
        });

        match row {
            Ok(row) => Ok(WorkerMetadata {
                agent_name: row.agent_name,
                tier: PromotionTier::from_name(&row.isolation_type)
                    .unwrap_or(PromotionTier::TaskWorker),
                container_id: None,
                service_name: None,
                username: None,
                home_path: Some(PathBuf::from(row.home_path)),
                created_at: row.created_at,
                promoted_at: None,
            }),
            Err(rusqlite::Error::QueryReturnedNoRows) => Err(SandboxError::NotFound(format!(
                "Worker {} not found",
                agent_name
            ))),
            Err(e) => Err(SandboxError::Internal(format!("Database error: {}", e))),
        }
    }

    /// List all workers.
    pub fn list_workers(&self) -> Result<Vec<WorkerMetadata>, SandboxError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, agent_name, isolation_type, home_path, shell_config, cache_dirs, network_egress, resource_limits, sudo_policy, mount_scopes, created_at, active FROM agent_sandboxes WHERE active = 1 ORDER BY created_at DESC"
        )?;

        let rows = stmt.query_map(rusqlite::params![], |row| {
            Ok(SandboxRow {
                id: row.get(0)?,
                agent_name: row.get(1)?,
                isolation_type: row.get(2)?,
                home_path: row.get(3)?,
                shell_config: row.get(4)?,
                cache_dirs: row.get(5)?,
                network_egress: row.get(6)?,
                resource_limits: row.get(7)?,
                sudo_policy: row.get(8)?,
                mount_scopes: row.get(9)?,
                created_at: row.get(10)?,
                active: row.get(11)?,
            })
        })?;

        let mut workers = Vec::new();
        for row in rows {
            let sandbox = row?;
            workers.push(WorkerMetadata {
                agent_name: sandbox.agent_name,
                tier: PromotionTier::from_name(&sandbox.isolation_type)
                    .unwrap_or(PromotionTier::TaskWorker),
                container_id: None,
                service_name: None,
                username: None,
                home_path: Some(PathBuf::from(sandbox.home_path)),
                created_at: sandbox.created_at,
                promoted_at: None,
            });
        }

        Ok(workers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_task_worker() {
        let dir = tempdir().unwrap();
        let manager = PromotionManager::new(dir.path().to_path_buf()).unwrap();

        let worker = manager.create_task_worker("test-agent").unwrap();
        assert_eq!(worker.agent_name, "test-agent");
        assert_eq!(worker.tier, PromotionTier::TaskWorker);
    }

    #[test]
    fn test_promote_to_gig_worker() {
        let dir = tempdir().unwrap();
        let manager = PromotionManager::new(dir.path().to_path_buf()).unwrap();

        let worker = manager.create_task_worker("test-agent").unwrap();
        let promoted = manager.promote_to_gig_worker("test-agent").unwrap();
        assert_eq!(promoted.tier, PromotionTier::GigWorker);
    }

    #[test]
    fn test_promote_to_dinit_service() {
        let dir = tempdir().unwrap();
        let manager = PromotionManager::new(dir.path().to_path_buf()).unwrap();

        let worker = manager.create_task_worker("test-agent").unwrap();
        manager.promote_to_gig_worker("test-agent").unwrap();
        let promoted = manager.promote_to_dinit_service("test-agent").unwrap();
        assert_eq!(promoted.tier, PromotionTier::DinitService);
    }

    #[test]
    fn test_promote_to_shared_user() {
        let dir = tempdir().unwrap();
        let manager = PromotionManager::new(dir.path().to_path_buf()).unwrap();

        let worker = manager.create_task_worker("test-agent").unwrap();
        manager.promote_to_gig_worker("test-agent").unwrap();
        manager.promote_to_dinit_service("test-agent").unwrap();
        let promoted = manager.promote_to_shared_user("test-agent").unwrap();
        assert_eq!(promoted.tier, PromotionTier::SharedUser);
    }

    #[test]
    fn test_promote_to_crabjar_user() {
        let dir = tempdir().unwrap();
        let manager = PromotionManager::new(dir.path().to_path_buf()).unwrap();

        let worker = manager.create_task_worker("test-agent").unwrap();
        manager.promote_to_gig_worker("test-agent").unwrap();
        manager.promote_to_dinit_service("test-agent").unwrap();
        manager.promote_to_shared_user("test-agent").unwrap();
        let promoted = manager.promote_to_crabjar_user("test-agent").unwrap();
        assert_eq!(promoted.tier, PromotionTier::CrabjarUser);
    }

    #[test]
    fn test_list_workers() {
        let dir = tempdir().unwrap();
        let manager = PromotionManager::new(dir.path().to_path_buf()).unwrap();

        manager.create_task_worker("agent-1").unwrap();
        manager.create_task_worker("agent-2").unwrap();

        let workers = manager.list_workers().unwrap();
        assert_eq!(workers.len(), 2);
    }
}