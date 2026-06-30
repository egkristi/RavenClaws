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

use rusqlite::{params, Connection, Result as SqlResult};
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
            "SELECT session_id, system_prompt, created_at, updated_at, total_tokens, message_count
             FROM sessions ORDER BY updated_at DESC",
        )?;
        let sessions = stmt
            .query_map([], |row| {
                Ok(StoredSession {
                    session_id: row.get(0)?,
                    system_prompt: row.get(1)?,
                    created_at: row.get(2)?,
                    updated_at: row.get(3)?,
                    total_tokens: row.get(4)?,
                    message_count: row.get(5)?,
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
}
