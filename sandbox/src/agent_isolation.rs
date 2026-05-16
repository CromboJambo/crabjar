use crate::error::SandboxError;
use crate::schema::{create_agent_sandbox, init_db, list_all_sandboxes, query_agent_sandbox};
use path_absolutize::Absolutize;
use rusqlite::Connection;
use tracing::{debug, info, warn};

/// Agent isolation tooling for per-agent Unix user creation, systemd-nspawn containers, and cgroup resource limits.
///
/// Practical hierarchy:
/// 1. Separate Unix user — best default. Gives real filesystem/user separation.
/// 2. Container or systemd-nspawn — stronger than a user account, lighter than a full VM.
/// 3. VM — best for high-risk autonomous work.
pub struct AgentIsolation<'a> {
    conn: &'a Connection,
}

impl<'a> AgentIsolation<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Initialize the sandbox database.
    pub fn init(&self) -> Result<(), SandboxError> {
        init_db(self.conn).map_err(SandboxError::Schema)
    }

    /// Create an agent sandbox configuration.
    pub fn create_sandbox(
        &self,
        agent_name: &str,
        isolation_type: &str,
        home_path: &str,
        shell_config: &str,
        cache_dirs: &str,
        network_egress: &str,
        resource_limits: &str,
        sudo_policy: &str,
        mount_scopes: &str,
    ) -> Result<String, SandboxError> {
        create_agent_sandbox(
            self.conn,
            agent_name,
            isolation_type,
            home_path,
            shell_config,
            cache_dirs,
            network_egress,
            resource_limits,
            sudo_policy,
            mount_scopes,
        )
        .map_err(SandboxError::Schema)
    }

    /// Query an agent sandbox configuration.
    pub fn query_sandbox(
        &self,
        agent_name: &str,
    ) -> Result<Option<crate::schema::SandboxRow>, SandboxError> {
        query_agent_sandbox(self.conn, agent_name).map_err(SandboxError::Schema)
    }

    /// List all agent sandbox configurations.
    pub fn list_sandboxes(
        &self,
        limit: usize,
    ) -> Result<Vec<crate::schema::SandboxRow>, SandboxError> {
        list_all_sandboxes(self.conn, limit).map_err(SandboxError::Schema)
    }

    /// Generate systemd-nspawn container configuration.
    pub fn generate_nspawn_config(
        &self,
        agent_name: &str,
        mount_scopes: &[String],
        resource_limits: &str,
    ) -> Result<String, SandboxError> {
        let mut config = String::new();
        config.push_str(&format!("[Container]\n"));
        config.push_str(&format!("HostName = {}\n", agent_name));
        config.push_str(&format!("NetworkInterface = {}\n", agent_name));

        for mount in mount_scopes {
            config.push_str(&format!("Bind = {}\n", mount));
        }

        config.push_str(&format!("Capability = {}\n", resource_limits));

        debug!(
            agent_name = %agent_name,
            "Agent isolation: systemd-nspawn config generated"
        );

        Ok(config)
    }

    /// Generate cgroup resource limits configuration.
    pub fn generate_cgroup_config(
        &self,
        agent_name: &str,
        resource_limits: &str,
    ) -> Result<String, SandboxError> {
        let mut config = String::new();

        let parts: Vec<&str> = resource_limits.split(',').collect();
        for part in parts {
            let (key, value) = part.split_once('=').unwrap_or((part, ""));
            config.push_str(&format!("{}.{} = {}\n", key, agent_name, value));
        }

        debug!(
            agent_name = %agent_name,
            resource_limits = %resource_limits,
            "Agent isolation: cgroup config generated"
        );

        Ok(config)
    }

    /// Verify sandbox path existence.
    pub fn verify_home_path(&self, home_path: &str) -> Result<bool, SandboxError> {
        let abs_path = std::path::Path::new(home_path).absolutize()?;
        Ok(abs_path.exists())
    }

    /// Verify mount scope existence.
    pub fn verify_mount_scope(&self, scope: &str) -> Result<bool, SandboxError> {
        let abs_path = std::path::Path::new(scope).absolutize()?;
        Ok(abs_path.exists())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_agent_isolation_init() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("sandbox.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();

        let isolation = AgentIsolation::new(&conn);
        isolation.init().unwrap();

        let rows = isolation.list_sandboxes(10).unwrap();
        assert_eq!(rows.len(), 0);
    }

    #[test]
    fn test_create_and_query_sandbox() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("sandbox.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();

        let isolation = AgentIsolation::new(&conn);
        isolation.init().unwrap();

        let id = isolation
            .create_sandbox(
                "test-agent",
                "unix_user",
                "/home/test-agent",
                "bash",
                "/home/test-agent/.cache",
                "restricted",
                "cpu=1,memory=2g",
                "no_sudo",
                "/repo:/tmp",
            )
            .unwrap();

        let row = isolation.query_sandbox("test-agent").unwrap();
        assert!(row.is_some());
        assert_eq!(row.unwrap().agent_name, "test-agent");
    }

    #[test]
    fn test_generate_nspawn_config() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("sandbox.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();

        let isolation = AgentIsolation::new(&conn);
        isolation.init().unwrap();

        let config = isolation
            .generate_nspawn_config(
                "test-agent",
                &[dir.path().to_string_lossy().to_string(), "/tmp".to_string()],
                "cpu=1,memory=2g",
            )
            .unwrap();

        assert!(config.contains("[Container]"));
        assert!(config.contains("HostName = test-agent"));
    }

    #[test]
    fn test_generate_cgroup_config() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("sandbox.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();

        let isolation = AgentIsolation::new(&conn);
        isolation.init().unwrap();

        let config = isolation
            .generate_cgroup_config("test-agent", "cpu=1,memory=2g")
            .unwrap();

        assert!(config.contains("cpu.test-agent"));
        assert!(config.contains("memory.test-agent"));
    }

    #[test]
    fn test_verify_home_path() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("sandbox.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();

        let isolation = AgentIsolation::new(&conn);
        isolation.init().unwrap();

        let exists = isolation
            .verify_home_path(dir.path().to_string_lossy().as_ref())
            .unwrap();
        assert!(exists);
    }
}
