//! # Conversation Persistence (SQLite backend)
//!
//! Provides SQLite-backed storage for conversation history so agents survive
//! pod restarts without losing context. Supports configurable retention policies
//! (time-based, count-based, token-budget-based).
//!
//! ## Architecture
//!
//! - `ConversationStore` — manages a SQLite database with sessions and messages tables
//! - Sessions are identified by a session ID (UUID or user-provided)
//! - Messages are stored with role, content, timestamp, and token count
//! - Retention policies are applied on read (not on write) for simplicity
//!
//! ## Usage
//!
//! ```rust,no_run
//! use ravenclaws::persistence::ConversationStore;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let store = ConversationStore::open(":memory:")?;
//! store.create_session("session-1", "You are a helpful assistant.")?;
//! store.add_message("session-1", "user", "Hello!", None)?;
//! let history = store.get_history("session-1", None)?;
//! # Ok(())
//! # }
//! ```

use rusqlite::{params, Connection, OptionalExtension, Result as SqlResult};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// A stored conversation message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredMessage {
    /// Message role (system, user, assistant, tool)
    pub role: String,
    /// Message content
    pub content: String,
    /// Unix timestamp when the message was created
    pub created_at: u64,
    /// Optional token count for budget tracking
    pub token_count: Option<u64>,
}

/// A stored conversation session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredSession {
    /// Unique session identifier
    pub session_id: String,
    /// Human-readable title (auto-generated or user-set)
    pub title: String,
    /// System prompt used for this session
    pub system_prompt: String,
    /// Unix timestamp when the session was created
    pub created_at: u64,
    /// Unix timestamp of the last activity
    pub updated_at: u64,
    /// Total token count across all messages
    pub total_tokens: u64,
    /// Number of messages in the session
    pub message_count: u64,
}

/// Retention policy for pruning old conversations
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum RetentionPolicy {
    /// Keep messages newer than this duration
    TimeBased(Duration),
    /// Keep at most this many messages (oldest removed first)
    CountBased(usize),
    /// Keep messages until total tokens exceed this budget
    TokenBudget(u64),
    /// No retention limit
    Unlimited,
}

impl RetentionPolicy {
    /// Apply this policy to a list of messages, returning the pruned list
    pub fn apply(&self, messages: &mut Vec<StoredMessage>) {
        match self {
            RetentionPolicy::TimeBased(duration) => {
                let cutoff = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
                    - duration.as_secs();
                messages.retain(|m| m.created_at >= cutoff);
            }
            RetentionPolicy::CountBased(max) => {
                if messages.len() > *max {
                    // Keep the most recent `max` messages
                    let keep = messages.split_off(messages.len() - max);
                    *messages = keep;
                }
            }
            RetentionPolicy::TokenBudget(budget) => {
                let mut total: u64 = 0;
                // Keep messages from newest to oldest until budget is exceeded
                messages.reverse();
                messages.retain(|m| {
                    let tokens = m.token_count.unwrap_or(0);
                    if total + tokens <= *budget {
                        total += tokens;
                        true
                    } else {
                        false
                    }
                });
                messages.reverse();
            }
            RetentionPolicy::Unlimited => {
                // No pruning
            }
        }
    }
}

/// SQLite-backed conversation store
#[derive(Debug)]
pub struct ConversationStore {
    conn: Connection,
}

impl ConversationStore {
    /// Open or create a SQLite database at the given path.
    /// Use `:memory:` for an in-memory database (useful for testing).
    pub fn open<P: AsRef<Path>>(path: P) -> SqlResult<Self> {
        let conn = Connection::open(path)?;
        let store = Self { conn };
        store.initialize_tables()?;
        Ok(store)
    }

    /// Initialize the database schema
    fn initialize_tables(&self) -> SqlResult<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS sessions (
                session_id   TEXT PRIMARY KEY,
                title        TEXT NOT NULL DEFAULT '',
                system_prompt TEXT NOT NULL DEFAULT '',
                created_at   INTEGER NOT NULL,
                updated_at   INTEGER NOT NULL,
                total_tokens INTEGER NOT NULL DEFAULT 0,
                message_count INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS messages (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id  TEXT NOT NULL,
                role        TEXT NOT NULL,
                content     TEXT NOT NULL,
                created_at  INTEGER NOT NULL,
                token_count INTEGER DEFAULT NULL,
                FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_messages_session_id ON messages(session_id);
            CREATE INDEX IF NOT EXISTS idx_messages_created_at ON messages(created_at);
            CREATE INDEX IF NOT EXISTS idx_sessions_updated_at ON sessions(updated_at);
            ",
        )?;
        Ok(())
    }

    /// Create a new conversation session
    pub fn create_session(&self, session_id: &str, system_prompt: &str) -> SqlResult<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.conn.execute(
            "INSERT OR IGNORE INTO sessions (session_id, system_prompt, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?3)",
            params![session_id, system_prompt, now],
        )?;
        Ok(())
    }

    /// Delete a session and all its messages
    pub fn delete_session(&self, session_id: &str) -> SqlResult<()> {
        self.conn.execute(
            "DELETE FROM messages WHERE session_id = ?1",
            params![session_id],
        )?;
        self.conn.execute(
            "DELETE FROM sessions WHERE session_id = ?1",
            params![session_id],
        )?;
        Ok(())
    }

    /// List all sessions, ordered by most recently updated first
    pub fn list_sessions(&self) -> SqlResult<Vec<StoredSession>> {
        let mut stmt = self.conn.prepare(
            "SELECT session_id, title, system_prompt, created_at, updated_at, total_tokens, message_count
             FROM sessions ORDER BY updated_at DESC",
        )?;
        let sessions = stmt
            .query_map([], |row| {
                Ok(StoredSession {
                    session_id: row.get(0)?,
                    title: row.get(1)?,
                    system_prompt: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                    total_tokens: row.get(5)?,
                    message_count: row.get(6)?,
                })
            })?
            .collect::<SqlResult<Vec<_>>>()?;
        Ok(sessions)
    }

    /// Add a message to a session
    pub fn add_message(
        &self,
        session_id: &str,
        role: &str,
        content: &str,
        token_count: Option<u64>,
    ) -> SqlResult<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Insert the message
        self.conn.execute(
            "INSERT INTO messages (session_id, role, content, created_at, token_count)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![session_id, role, content, now, token_count],
        )?;

        // Update session metadata
        self.conn.execute(
            "UPDATE sessions SET
                updated_at = ?1,
                total_tokens = total_tokens + ?2,
                message_count = message_count + 1
             WHERE session_id = ?3",
            params![now, token_count.unwrap_or(0), session_id],
        )?;

        Ok(())
    }

    /// Get message history for a session, optionally applying a retention policy
    pub fn get_history(
        &self,
        session_id: &str,
        policy: Option<RetentionPolicy>,
    ) -> SqlResult<Vec<StoredMessage>> {
        let mut stmt = self.conn.prepare(
            "SELECT role, content, created_at, token_count
             FROM messages WHERE session_id = ?1
             ORDER BY created_at ASC",
        )?;

        let mut messages: Vec<StoredMessage> = stmt
            .query_map(params![session_id], |row| {
                Ok(StoredMessage {
                    role: row.get(0)?,
                    content: row.get(1)?,
                    created_at: row.get(2)?,
                    token_count: row.get(3)?,
                })
            })?
            .collect::<SqlResult<Vec<_>>>()?;

        // Apply retention policy if specified
        if let Some(policy) = policy {
            policy.apply(&mut messages);
        }

        Ok(messages)
    }

    /// Get the number of messages in a session
    pub fn message_count(&self, session_id: &str) -> SqlResult<u64> {
        let count: u64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM messages WHERE session_id = ?1",
                params![session_id],
                |row| row.get(0),
            )
            .unwrap_or(0);
        Ok(count)
    }

    /// Get the total token count for a session
    pub fn total_tokens(&self, session_id: &str) -> SqlResult<u64> {
        let total: u64 = self
            .conn
            .query_row(
                "SELECT COALESCE(SUM(token_count), 0) FROM messages WHERE session_id = ?1",
                params![session_id],
                |row| row.get(0),
            )
            .unwrap_or(0);
        Ok(total)
    }

    /// Prune old sessions based on a retention policy applied to session age
    pub fn prune_sessions(&self, max_age: Duration) -> SqlResult<u64> {
        let cutoff = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            - max_age.as_secs();

        // Find sessions to delete
        let sessions: Vec<String> = self
            .conn
            .prepare("SELECT session_id FROM sessions WHERE updated_at < ?1")?
            .query_map(params![cutoff], |row| row.get(0))?
            .collect::<SqlResult<Vec<_>>>()?;

        let count = sessions.len() as u64;
        for session_id in &sessions {
            self.delete_session(session_id)?;
        }

        Ok(count)
    }

    /// Convert stored messages to `ChatMessage` format for the LLM
    pub fn to_chat_messages(
        &self,
        session_id: &str,
        policy: Option<RetentionPolicy>,
    ) -> SqlResult<Vec<crate::llm::ChatMessage>> {
        let stored = self.get_history(session_id, policy)?;
        Ok(stored
            .into_iter()
            .map(|m| crate::llm::ChatMessage {
                role: m.role,
                content: m.content,
                content_parts: None,
            })
            .collect())
    }

    /// Import messages from a `ConversationMemory` into a session
    pub fn import_memory(
        &self,
        session_id: &str,
        memory: &crate::agent::ConversationMemory,
        system_prompt: &str,
    ) -> SqlResult<()> {
        self.create_session(session_id, system_prompt)?;

        for msg in memory.history() {
            self.add_message(session_id, &msg.role, &msg.content, None)?;
        }

        Ok(())
    }

    /// Set an explicit title for a session.
    pub fn set_title(&self, session_id: &str, title: &str) -> SqlResult<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.conn.execute(
            "UPDATE sessions SET title = ?1, updated_at = ?2 WHERE session_id = ?3",
            params![title, now, session_id],
        )?;
        Ok(())
    }

    /// Get the title of a session (empty string if untitled).
    pub fn get_title(&self, session_id: &str) -> SqlResult<String> {
        let title: String = self
            .conn
            .query_row(
                "SELECT title FROM sessions WHERE session_id = ?1",
                params![session_id],
                |row| row.get(0),
            )
            .unwrap_or_default();
        Ok(title)
    }

    /// Auto-title a session from its first user message (truncated to `max_len`).
    ///
    /// Returns the assigned title, or `None` if the session has no user message.
    pub fn auto_title(&self, session_id: &str, max_len: usize) -> SqlResult<Option<String>> {
        let first: Option<String> = self
            .conn
            .query_row(
                "SELECT content FROM messages WHERE session_id = ?1 AND role = 'user'
                 ORDER BY created_at ASC LIMIT 1",
                params![session_id],
                |row| row.get(0),
            )
            .optional()?;

        let Some(first) = first else {
            return Ok(None);
        };

        let title = truncate_to_char_boundary(&first, max_len);
        self.set_title(session_id, &title)?;
        Ok(Some(title))
    }

    /// Search conversations by keyword across titles, system prompts, and message
    /// content. Returns matching session IDs (deduplicated), most recently
    /// updated first.
    pub fn search_conversations(&self, query: &str) -> SqlResult<Vec<StoredSession>> {
        let pattern = format!("%{}%", query);
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT s.session_id, s.title, s.system_prompt, s.created_at, s.updated_at,
                    s.total_tokens, s.message_count
             FROM sessions s
             LEFT JOIN messages m ON m.session_id = s.session_id
             WHERE s.title LIKE ?1 COLLATE NOCASE
                OR s.system_prompt LIKE ?1 COLLATE NOCASE
                OR m.content LIKE ?1 COLLATE NOCASE
             ORDER BY s.updated_at DESC",
        )?;

        let results = stmt
            .query_map(params![pattern], |row| {
                Ok(StoredSession {
                    session_id: row.get(0)?,
                    title: row.get(1)?,
                    system_prompt: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                    total_tokens: row.get(5)?,
                    message_count: row.get(6)?,
                })
            })?
            .collect::<SqlResult<Vec<_>>>()?;
        Ok(results)
    }
}

/// Truncate a string to at most `max_len` bytes, breaking on a UTF-8 character
/// boundary (never splitting a multi-byte character).
fn truncate_to_char_boundary(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        return s.to_string();
    }
    let mut end = max_len;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    s[..end].to_string()
}

/// A single long-term memory entry (key-value store).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// Memory key (unique within its scope)
    pub key: String,
    /// Memory value
    pub value: String,
    /// Scope (e.g. "user", "global", "project:<id>")
    pub scope: String,
    /// Unix timestamp when the memory was created
    pub created_at: u64,
    /// Unix timestamp of the last update
    pub updated_at: u64,
}

/// Long-term memory store backed by SQLite.
///
/// Provides key-value persistence with upsert semantics and scoping, so agents
/// can retain durable facts about users and projects across sessions — the
/// "long-term memory" primitive used by OpenClaw/Manus-style assistants.
///
/// # Usage
///
/// ```rust,no_run
/// use ravenclaws::persistence::MemoryStore;
///
/// let store = MemoryStore::open(":memory:").expect("open memory store");
/// store.set("user", "favorite_color", "blue").expect("set");
/// assert_eq!(store.get("user", "favorite_color").unwrap(), Some("blue".to_string()));
/// ```
#[derive(Debug)]
pub struct MemoryStore {
    conn: Connection,
}

impl MemoryStore {
    /// Open or create a SQLite database at the given path for memory storage.
    /// Use `:memory:` for an in-memory database (useful for testing).
    pub fn open<P: AsRef<Path>>(path: P) -> SqlResult<Self> {
        let conn = Connection::open(path)?;
        let store = Self { conn };
        store.initialize_tables()?;
        Ok(store)
    }

    fn initialize_tables(&self) -> SqlResult<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS memories (
                key        TEXT NOT NULL,
                scope      TEXT NOT NULL,
                value      TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                PRIMARY KEY (scope, key)
            );

            CREATE INDEX IF NOT EXISTS idx_memories_scope ON memories(scope);
            ",
        )?;
        Ok(())
    }

    /// Set (upsert) a memory value for a key within a scope.
    pub fn set(&self, scope: &str, key: &str, value: &str) -> SqlResult<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.conn.execute(
            "INSERT INTO memories (key, scope, value, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?4)
             ON CONFLICT(scope, key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![key, scope, value, now],
        )?;
        Ok(())
    }

    /// Get a memory value for a key within a scope.
    pub fn get(&self, scope: &str, key: &str) -> SqlResult<Option<String>> {
        let value: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM memories WHERE scope = ?1 AND key = ?2",
                params![scope, key],
                |row| row.get(0),
            )
            .optional()?;
        Ok(value)
    }

    /// Delete a memory entry.
    pub fn delete(&self, scope: &str, key: &str) -> SqlResult<()> {
        self.conn.execute(
            "DELETE FROM memories WHERE scope = ?1 AND key = ?2",
            params![scope, key],
        )?;
        Ok(())
    }

    /// List all memories in a scope (optionally all scopes if `scope` is `None`).
    pub fn list(&self, scope: Option<&str>) -> SqlResult<Vec<MemoryEntry>> {
        let mut stmt = if scope.is_some() {
            self.conn.prepare(
                "SELECT key, value, scope, created_at, updated_at
                 FROM memories WHERE scope = ?1 ORDER BY updated_at DESC",
            )?
        } else {
            self.conn.prepare(
                "SELECT key, value, scope, created_at, updated_at
                 FROM memories ORDER BY scope, updated_at DESC",
            )?
        };

        let entries = if scope.is_some() {
            stmt.query_map(params![scope], |row| {
                Ok(MemoryEntry {
                    key: row.get(0)?,
                    value: row.get(1)?,
                    scope: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            })?
            .collect::<SqlResult<Vec<_>>>()?
        } else {
            stmt.query_map([], |row| {
                Ok(MemoryEntry {
                    key: row.get(0)?,
                    value: row.get(1)?,
                    scope: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            })?
            .collect::<SqlResult<Vec<_>>>()?
        };

        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn create_test_store() -> ConversationStore {
        ConversationStore::open(":memory:").expect("Failed to create in-memory store")
    }

    #[test]
    fn test_create_and_list_sessions() {
        let store = create_test_store();
        store.create_session("test-1", "You are helpful.").unwrap();
        store.create_session("test-2", "You are a poet.").unwrap();

        let sessions = store.list_sessions().unwrap();
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].session_id, "test-2"); // most recent first
        assert_eq!(sessions[1].session_id, "test-1");
    }

    #[test]
    fn test_add_and_get_messages() {
        let store = create_test_store();
        store
            .create_session("session-1", "You are helpful.")
            .unwrap();
        store
            .add_message("session-1", "user", "Hello!", Some(5))
            .unwrap();
        store
            .add_message("session-1", "assistant", "Hi there!", Some(10))
            .unwrap();

        let history = store.get_history("session-1", None).unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].role, "user");
        assert_eq!(history[0].content, "Hello!");
        assert_eq!(history[0].token_count, Some(5));
        assert_eq!(history[1].role, "assistant");
        assert_eq!(history[1].content, "Hi there!");
        assert_eq!(history[1].token_count, Some(10));
    }

    #[test]
    fn test_message_count_and_tokens() {
        let store = create_test_store();
        store
            .create_session("session-1", "You are helpful.")
            .unwrap();
        store
            .add_message("session-1", "user", "Hello!", Some(5))
            .unwrap();
        store
            .add_message("session-1", "assistant", "Hi!", Some(3))
            .unwrap();

        assert_eq!(store.message_count("session-1").unwrap(), 2);
        assert_eq!(store.total_tokens("session-1").unwrap(), 8);
    }

    #[test]
    fn test_delete_session() {
        let store = create_test_store();
        store
            .create_session("session-1", "You are helpful.")
            .unwrap();
        store
            .add_message("session-1", "user", "Hello!", None)
            .unwrap();

        store.delete_session("session-1").unwrap();
        let sessions = store.list_sessions().unwrap();
        assert_eq!(sessions.len(), 0);
        assert_eq!(store.message_count("session-1").unwrap(), 0);
    }

    #[test]
    fn test_retention_policy_time_based() {
        let mut messages = vec![
            StoredMessage {
                role: "user".into(),
                content: "old".into(),
                created_at: 1000,
                token_count: None,
            },
            StoredMessage {
                role: "user".into(),
                content: "new".into(),
                created_at: u64::MAX,
                token_count: None,
            },
        ];

        // Keep messages newer than 1 hour
        let policy = RetentionPolicy::TimeBased(Duration::from_secs(3600));
        policy.apply(&mut messages);

        // Only the "new" message (with far-future timestamp) should remain
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "new");
    }

    #[test]
    fn test_retention_policy_count_based() {
        let mut messages: Vec<StoredMessage> = (0..10)
            .map(|i| StoredMessage {
                role: "user".into(),
                content: format!("msg-{}", i),
                created_at: i as u64,
                token_count: None,
            })
            .collect();

        let policy = RetentionPolicy::CountBased(3);
        policy.apply(&mut messages);

        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].content, "msg-7");
        assert_eq!(messages[2].content, "msg-9");
    }

    #[test]
    fn test_retention_policy_token_budget() {
        let mut messages = vec![
            StoredMessage {
                role: "user".into(),
                content: "a".into(),
                created_at: 1,
                token_count: Some(100),
            },
            StoredMessage {
                role: "user".into(),
                content: "b".into(),
                created_at: 2,
                token_count: Some(50),
            },
            StoredMessage {
                role: "user".into(),
                content: "c".into(),
                created_at: 3,
                token_count: Some(30),
            },
        ];

        // Budget of 80 tokens — should keep newest messages up to 80 tokens
        let policy = RetentionPolicy::TokenBudget(80);
        policy.apply(&mut messages);

        // From newest: c(30) + b(50) = 80, a(100) exceeds budget
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].content, "b");
        assert_eq!(messages[1].content, "c");
    }

    #[test]
    fn test_retention_policy_unlimited() {
        let mut messages = vec![
            StoredMessage {
                role: "user".into(),
                content: "a".into(),
                created_at: 1,
                token_count: None,
            },
            StoredMessage {
                role: "user".into(),
                content: "b".into(),
                created_at: 2,
                token_count: None,
            },
        ];

        let policy = RetentionPolicy::Unlimited;
        policy.apply(&mut messages);
        assert_eq!(messages.len(), 2);
    }

    #[test]
    fn test_prune_sessions() {
        let store = create_test_store();
        store.create_session("old-session", "Old.").unwrap();
        store.create_session("new-session", "New.").unwrap();

        // Manually set old session's updated_at to the past
        let past = 1000; // year 1970
        store
            .conn
            .execute(
                "UPDATE sessions SET updated_at = ?1 WHERE session_id = 'old-session'",
                params![past],
            )
            .unwrap();

        let pruned = store.prune_sessions(Duration::from_secs(3600)).unwrap();
        assert_eq!(pruned, 1);

        let sessions = store.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].session_id, "new-session");
    }

    #[test]
    fn test_to_chat_messages() {
        let store = create_test_store();
        store.create_session("s1", "System prompt.").unwrap();
        store
            .add_message("s1", "system", "System prompt.", None)
            .unwrap();
        store.add_message("s1", "user", "Hello!", None).unwrap();

        let chat_msgs = store.to_chat_messages("s1", None).unwrap();
        assert_eq!(chat_msgs.len(), 2);
        assert_eq!(chat_msgs[0].role, "system");
        assert_eq!(chat_msgs[1].content, "Hello!");
    }

    #[test]
    fn test_import_memory() {
        let store = create_test_store();
        let mut memory = crate::agent::ConversationMemory::new("System prompt.", 0);
        memory.add_user_message("Hello!");
        memory.add_assistant_message("Hi there!");

        store
            .import_memory("imported-session", &memory, "System prompt.")
            .unwrap();

        let history = store.get_history("imported-session", None).unwrap();
        assert_eq!(history.len(), 3); // system + user + assistant
        assert_eq!(history[0].content, "System prompt.");
        assert_eq!(history[1].content, "Hello!");
        assert_eq!(history[2].content, "Hi there!");
    }

    #[test]
    fn test_session_metadata_updates() {
        let store = create_test_store();
        store.create_session("s1", "Helpful assistant.").unwrap();

        store.add_message("s1", "user", "Hi", Some(3)).unwrap();
        store
            .add_message("s1", "assistant", "Hello!", Some(5))
            .unwrap();

        let sessions = store.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].message_count, 2);
        assert_eq!(sessions[0].total_tokens, 8);
    }

    #[test]
    fn test_nonexistent_session_returns_empty() {
        let store = create_test_store();
        let history = store.get_history("nonexistent", None).unwrap();
        assert!(history.is_empty());
        assert_eq!(store.message_count("nonexistent").unwrap(), 0);
        assert_eq!(store.total_tokens("nonexistent").unwrap(), 0);
    }

    // ── Auto-title tests ───────────────────────────────────────────────────

    #[test]
    fn test_auto_title_from_first_user_message() {
        let store = create_test_store();
        store.create_session("s1", "System.").unwrap();
        store
            .add_message("s1", "user", "Hello there friend", None)
            .unwrap();
        store.add_message("s1", "assistant", "Hi!", None).unwrap();

        let title = store.auto_title("s1", 40).unwrap().unwrap();
        assert_eq!(title, "Hello there friend");
        assert_eq!(store.get_title("s1").unwrap(), "Hello there friend");
    }

    #[test]
    fn test_auto_title_truncates_to_max_len() {
        let store = create_test_store();
        store.create_session("s1", "System.").unwrap();
        store
            .add_message("s1", "user", "This is a very long first message", None)
            .unwrap();

        let title = store.auto_title("s1", 10).unwrap().unwrap();
        assert_eq!(title, "This is a ");
        assert_eq!(store.get_title("s1").unwrap(), "This is a ");
    }

    #[test]
    fn test_auto_title_no_user_message_returns_none() {
        let store = create_test_store();
        store.create_session("s1", "System.").unwrap();
        assert!(store.auto_title("s1", 40).unwrap().is_none());
    }

    #[test]
    fn test_set_and_get_title() {
        let store = create_test_store();
        store.create_session("s1", "System.").unwrap();
        store.set_title("s1", "My custom title").unwrap();
        assert_eq!(store.get_title("s1").unwrap(), "My custom title");
    }

    // ── Search tests ───────────────────────────────────────────────────────

    #[test]
    fn test_search_by_message_content() {
        let store = create_test_store();
        store.create_session("s1", "System.").unwrap();
        store
            .add_message("s1", "user", "The capital of Norway", None)
            .unwrap();
        store.create_session("s2", "System.").unwrap();
        store
            .add_message("s2", "user", "Something unrelated", None)
            .unwrap();

        let results = store.search_conversations("Norway").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].session_id, "s1");
    }

    #[test]
    fn test_search_by_title() {
        let store = create_test_store();
        store.create_session("s1", "System.").unwrap();
        store.set_title("s1", "Deployment checklist").unwrap();
        store.create_session("s2", "System.").unwrap();

        let results = store.search_conversations("deployment").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].session_id, "s1");
    }

    #[test]
    fn test_search_no_match_returns_empty() {
        let store = create_test_store();
        store.create_session("s1", "System.").unwrap();
        store.add_message("s1", "user", "hello", None).unwrap();

        let results = store.search_conversations("zzzzz").unwrap();
        assert!(results.is_empty());
    }

    // ── MemoryStore tests ──────────────────────────────────────────────────

    fn create_test_memory_store() -> MemoryStore {
        MemoryStore::open(":memory:").expect("Failed to create in-memory memory store")
    }

    #[test]
    fn test_memory_set_and_get() {
        let store = create_test_memory_store();
        store.set("user", "name", "Alice").unwrap();
        assert_eq!(
            store.get("user", "name").unwrap(),
            Some("Alice".to_string())
        );
        assert_eq!(store.get("user", "missing").unwrap(), None);
    }

    #[test]
    fn test_memory_upsert() {
        let store = create_test_memory_store();
        store.set("user", "name", "Alice").unwrap();
        store.set("user", "name", "Bob").unwrap();
        assert_eq!(store.get("user", "name").unwrap(), Some("Bob".to_string()));
    }

    #[test]
    fn test_memory_scoped() {
        let store = create_test_memory_store();
        store.set("user", "name", "Alice").unwrap();
        store.set("project:1", "name", "Bob").unwrap();
        // Same key, different scopes are independent
        assert_eq!(
            store.get("user", "name").unwrap(),
            Some("Alice".to_string())
        );
        assert_eq!(
            store.get("project:1", "name").unwrap(),
            Some("Bob".to_string())
        );
    }

    #[test]
    fn test_memory_delete() {
        let store = create_test_memory_store();
        store.set("user", "name", "Alice").unwrap();
        store.delete("user", "name").unwrap();
        assert_eq!(store.get("user", "name").unwrap(), None);
    }

    #[test]
    fn test_memory_list() {
        let store = create_test_memory_store();
        store.set("user", "a", "1").unwrap();
        store.set("user", "b", "2").unwrap();
        store.set("global", "c", "3").unwrap();

        let user_entries = store.list(Some("user")).unwrap();
        assert_eq!(user_entries.len(), 2);

        let all_entries = store.list(None).unwrap();
        assert_eq!(all_entries.len(), 3);
    }
}
