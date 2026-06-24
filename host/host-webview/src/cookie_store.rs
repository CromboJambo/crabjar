/// SQLite-backed cookie and session store.
///
/// Replaces Electron's `session.cookies.get/set/remove` API.
/// Persists cookies, session tokens, and partition data to a local SQLite database.
use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::Utc;

/// A persisted cookie entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub expires: Option<i64>,
    pub secure: bool,
    pub http_only: bool,
    pub same_site: String,
}

/// A persisted session token (OAuth2 / MSAL).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionToken {
    pub key: String,
    pub value: String,
    #[serde(default)]
    pub encrypted: bool,
    pub created_at: i64,
    pub expires_at: Option<i64>,
}

/// Partition metadata for per-profile isolation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Partition {
    pub name: String,
    pub zoom_level: f64,
    pub created_at: i64,
}

/// SQLite-backed persistence layer for cookies, tokens, and partitions.
#[allow(clippy::arc_with_non_send_sync)]
pub struct CookieStore {
    db_path: PathBuf,
    conn: Arc<RwLock<Connection>>,
}

impl CookieStore {
    /// Open or create the cookie store database.
    pub fn open(db_path: PathBuf) -> SqlResult<Self> {
        let conn = Connection::open(&db_path)?;
        Self::init(&conn)?;
        Ok(Self {
            db_path,
            conn: Arc::new(RwLock::new(conn)),
        })
    }

    fn init(conn: &Connection) -> SqlResult<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS cookies (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                value TEXT NOT NULL,
                domain TEXT NOT NULL,
                path TEXT NOT NULL DEFAULT '/',
                expires INTEGER,
                secure INTEGER NOT NULL DEFAULT 0,
                http_only INTEGER NOT NULL DEFAULT 0,
                same_site TEXT NOT NULL DEFAULT 'Lax',
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_cookies_domain ON cookies(domain);
            CREATE INDEX IF NOT EXISTS idx_cookies_name ON cookies(name);

            CREATE TABLE IF NOT EXISTS tokens (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                encrypted INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                expires_at INTEGER
            );

            CREATE TABLE IF NOT EXISTS partitions (
                name TEXT PRIMARY KEY,
                zoom_level REAL NOT NULL DEFAULT 0.0,
                created_at INTEGER NOT NULL
            );",
        )?;
        Ok(())
    }

    // --- Cookie CRUD ---

    pub async fn save_cookie(&self, cookie: &Cookie) -> SqlResult<Uuid> {
        let id = Uuid::new_v4();
        let now = Utc::now().timestamp();
        let tx = self.conn.write().await;
        tx.execute(
            "INSERT OR REPLACE INTO cookies (id, name, value, domain, path, expires, secure, http_only, same_site, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                id.to_string(), cookie.name, cookie.value, cookie.domain, cookie.path,
                cookie.expires, cookie.secure as i32, cookie.http_only as i32,
                cookie.same_site, now, now
            ],
        )?;
        Ok(id)
    }

    pub async fn get_cookies_by_domain(&self, domain: &str) -> SqlResult<Vec<Cookie>> {
        let tx = self.conn.read().await;
        let mut stmt = tx.prepare(
            "SELECT name, value, domain, path, expires, secure, http_only, same_site
             FROM cookies WHERE domain = ?1",
        )?;
        let cookies = stmt.query_map(params![domain], |row| {
            Ok(Cookie {
                name: row.get(0)?,
                value: row.get(1)?,
                domain: row.get(2)?,
                path: row.get(3)?,
                expires: row.get(4)?,
                secure: row.get::<_, i32>(5)? != 0,
                http_only: row.get::<_, i32>(6)? != 0,
                same_site: row.get(7)?,
            })
        })?;
        cookies.collect::<SqlResult<Vec<Cookie>>>()
    }

    pub async fn get_cookie(&self, domain: &str, name: &str) -> SqlResult<Option<Cookie>> {
        let tx = self.conn.read().await;
        let mut stmt = tx.prepare(
            "SELECT name, value, domain, path, expires, secure, http_only, same_site
             FROM cookies WHERE domain = ?1 AND name = ?2",
        )?;
        let mut rows = stmt.query(params![domain, name])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Cookie {
                name: row.get(0)?,
                value: row.get(1)?,
                domain: row.get(2)?,
                path: row.get(3)?,
                expires: row.get(4)?,
                secure: row.get::<_, i32>(5)? != 0,
                http_only: row.get::<_, i32>(6)? != 0,
                same_site: row.get(7)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn remove_cookie(&self, domain: &str, name: &str) -> SqlResult<bool> {
        let tx = self.conn.write().await;
        let rows = tx.execute(
            "DELETE FROM cookies WHERE domain = ?1 AND name = ?2",
            params![domain, name],
        )?;
        Ok(rows > 0)
    }

    pub async fn remove_cookies_by_domain(&self, domain: &str) -> SqlResult<usize> {
        let tx = self.conn.write().await;
        tx.execute("DELETE FROM cookies WHERE domain = ?1", params![domain])
    }

    pub async fn clear_all(&self) -> SqlResult<usize> {
        let tx = self.conn.write().await;
        tx.execute("DELETE FROM cookies", params![])?;
        tx.execute("DELETE FROM tokens", params![])?;
        tx.execute("DELETE FROM partitions", params![])?;
        Ok(0)
    }

    pub async fn list_cookies(&self) -> SqlResult<Vec<Cookie>> {
        let tx = self.conn.read().await;
        let mut stmt = tx.prepare(
            "SELECT name, value, domain, path, expires, secure, http_only, same_site
             FROM cookies ORDER BY domain, name",
        )?;
        let cookies = stmt.query_map(params![], |row| {
            Ok(Cookie {
                name: row.get(0)?,
                value: row.get(1)?,
                domain: row.get(2)?,
                path: row.get(3)?,
                expires: row.get(4)?,
                secure: row.get::<_, i32>(5)? != 0,
                http_only: row.get::<_, i32>(6)? != 0,
                same_site: row.get(7)?,
            })
        })?;
        cookies.collect::<SqlResult<Vec<Cookie>>>()
    }

    // --- Token CRUD ---

    pub async fn save_token(&self, token: &SessionToken) -> SqlResult<()> {
        let tx = self.conn.write().await;
        tx.execute(
            "INSERT OR REPLACE INTO tokens (key, value, encrypted, created_at, expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                token.key, token.value, token.encrypted as i32,
                token.created_at, token.expires_at
            ],
        )?;
        Ok(())
    }

    pub async fn get_token(&self, key: &str) -> SqlResult<Option<SessionToken>> {
        let tx = self.conn.read().await;
        let mut stmt = tx.prepare(
            "SELECT key, value, encrypted, created_at, expires_at FROM tokens WHERE key = ?1",
        )?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            Ok(Some(SessionToken {
                key: row.get(0)?,
                value: row.get(1)?,
                encrypted: row.get::<_, i32>(2)? != 0,
                created_at: row.get(3)?,
                expires_at: row.get(4)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn remove_token(&self, key: &str) -> SqlResult<bool> {
        let tx = self.conn.write().await;
        let rows = tx.execute("DELETE FROM tokens WHERE key = ?1", params![key])?;
        Ok(rows > 0)
    }

    pub async fn list_tokens(&self) -> SqlResult<Vec<SessionToken>> {
        let tx = self.conn.read().await;
        let mut stmt = tx.prepare(
            "SELECT key, value, encrypted, created_at, expires_at FROM tokens",
        )?;
        let tokens = stmt.query_map(params![], |row| {
            Ok(SessionToken {
                key: row.get(0)?,
                value: row.get(1)?,
                encrypted: row.get::<_, i32>(2)? != 0,
                created_at: row.get(3)?,
                expires_at: row.get(4)?,
            })
        })?;
        tokens.collect::<SqlResult<Vec<SessionToken>>>()
    }

    /// Remove expired tokens. Returns count of removed tokens.
    pub async fn cleanup_expired(&self) -> SqlResult<usize> {
        let now = Utc::now().timestamp();
        let tx = self.conn.write().await;
        let rows = tx.execute(
            "DELETE FROM tokens WHERE expires_at IS NOT NULL AND expires_at < ?1",
            params![now],
        )?;
        Ok(rows)
    }

    // --- Partition CRUD ---

    pub async fn save_partition(&self, partition: &Partition) -> SqlResult<()> {
        let tx = self.conn.write().await;
        tx.execute(
            "INSERT OR REPLACE INTO partitions (name, zoom_level, created_at)
             VALUES (?1, ?2, ?3)",
            params![partition.name, partition.zoom_level, partition.created_at],
        )?;
        Ok(())
    }

    pub async fn get_partition(&self, name: &str) -> SqlResult<Option<Partition>> {
        let tx = self.conn.read().await;
        let mut stmt = tx.prepare(
            "SELECT name, zoom_level, created_at FROM partitions WHERE name = ?1",
        )?;
        let mut rows = stmt.query(params![name])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Partition {
                name: row.get(0)?,
                zoom_level: row.get(1)?,
                created_at: row.get(2)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn list_partitions(&self) -> SqlResult<Vec<Partition>> {
        let tx = self.conn.read().await;
        let mut stmt = tx.prepare(
            "SELECT name, zoom_level, created_at FROM partitions ORDER BY name",
        )?;
        let partitions = stmt.query_map(params![], |row| {
            Ok(Partition {
                name: row.get(0)?,
                zoom_level: row.get(1)?,
                created_at: row.get(2)?,
            })
        })?;
        partitions.collect::<SqlResult<Vec<Partition>>>()
    }

    pub async fn remove_partition(&self, name: &str) -> SqlResult<bool> {
        let tx = self.conn.write().await;
        let rows = tx.execute("DELETE FROM partitions WHERE name = ?1", params![name])?;
        Ok(rows > 0)
    }

    /// Database path for external access.
    pub fn db_path(&self) -> &PathBuf {
        &self.db_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db() -> PathBuf {
        // Create a unique temp directory and return a path inside it
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cookies.db");
        // Leak the TempDir so the directory persists for the test
        let leaked = dir.into_path();
        leaked.join("cookies.db")
    }

    #[tokio::test]
    async fn test_save_and_get_cookie() {
        let store = CookieStore::open(temp_db()).unwrap();
        let cookie = Cookie {
            name: "session".into(),
            value: "abc123".into(),
            domain: "teams.microsoft.com".into(),
            path: "/".into(),
            expires: Some(9999999999),
            secure: true,
            http_only: true,
            same_site: "None".into(),
        };
        let id = store.save_cookie(&cookie).await.unwrap();
        assert!(!id.is_nil());

        let found = store.get_cookie("teams.microsoft.com", "session").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().value, "abc123");
    }

    #[tokio::test]
    async fn test_save_and_get_token() {
        let store = CookieStore::open(temp_db()).unwrap();
        let token = SessionToken {
            key: "msal.token.abc".into(),
            value: "encrypted_value_here".into(),
            encrypted: true,
            created_at: Utc::now().timestamp(),
            expires_at: Some(Utc::now().timestamp() + 3600),
        };
        store.save_token(&token).await.unwrap();

        let found = store.get_token("msal.token.abc").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().value, "encrypted_value_here");
    }

    #[tokio::test]
    async fn test_cleanup_expired_tokens() {
        let store = CookieStore::open(temp_db()).unwrap();
        let past = Utc::now().timestamp() - 86400; // 1 day ago
        let token = SessionToken {
            key: "expired.token".into(),
            value: "old".into(),
            encrypted: false,
            created_at: past,
            expires_at: Some(past),
        };
        store.save_token(&token).await.unwrap();

        let count = store.cleanup_expired().await.unwrap();
        assert_eq!(count, 1);

        let remaining = store.list_tokens().await.unwrap();
        assert!(remaining.is_empty(), "expected no tokens, got: {:?}", remaining);
    }

    #[tokio::test]
    async fn test_partition_crud() {
        let store = CookieStore::open(temp_db()).unwrap();
        let partition = Partition {
            name: "profile-teams".into(),
            zoom_level: 1.5,
            created_at: Utc::now().timestamp(),
        };
        store.save_partition(&partition).await.unwrap();

        let found = store.get_partition("profile-teams").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().zoom_level, 1.5);
    }
}
