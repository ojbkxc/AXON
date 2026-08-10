use std::sync::Arc;

use chrono::Utc;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use axon_core::{AxonError, Result, TokenUsage};

pub struct Store {
    pool: Pool<SqliteConnectionManager>,
}

impl Store {
    pub fn open(path: &str, max_connections: u32) -> Result<Arc<Self>> {
        let manager = SqliteConnectionManager::file(path);
        let pool = Pool::builder()
            .max_size(max_connections)
            .build(manager)
            .map_err(|e| AxonError::Storage(format!("pool init: {e}")))?;

        let store = Arc::new(Store { pool });
        store.init_schema()?;
        Ok(store)
    }

    fn init_schema(&self) -> Result<()> {
        let conn = self.conn()?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS conversations (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                title TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                metadata TEXT DEFAULT '{}'
            );

            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL,
                parent_id TEXT,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                tool_calls TEXT,
                tool_call_id TEXT,
                model TEXT,
                usage TEXT,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (conversation_id) REFERENCES conversations(id)
            );

            CREATE INDEX IF NOT EXISTS idx_messages_conv ON messages(conversation_id);
            CREATE INDEX IF NOT EXISTS idx_messages_parent ON messages(parent_id);

            CREATE TABLE IF NOT EXISTS agents (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                definition TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS usage_records (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                conversation_id TEXT,
                agent_id TEXT,
                model TEXT,
                prompt_tokens INTEGER,
                completion_tokens INTEGER,
                total_tokens INTEGER,
                duration_ms INTEGER,
                timestamp INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_usage_ts ON usage_records(timestamp);
            CREATE INDEX IF NOT EXISTS idx_usage_model ON usage_records(model);

            CREATE TABLE IF NOT EXISTS memory (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                namespace TEXT DEFAULT 'default',
                updated_at INTEGER NOT NULL
            );
            "#,
        )?;
        Ok(())
    }

    fn conn(&self) -> Result<r2d2::PooledConnection<SqliteConnectionManager>> {
        self.pool
            .get()
            .map_err(|e| AxonError::Storage(format!("pool get: {e}")))
    }

    pub fn create_conversation(&self, agent_id: &str, title: Option<&str>) -> Result<Conversation> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().timestamp();
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO conversations (id, agent_id, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, agent_id, title, now, now],
        )?;
        Ok(Conversation {
            id,
            agent_id: agent_id.into(),
            title: title.map(String::from),
            created_at: now,
            updated_at: now,
        })
    }

    pub fn get_conversation(&self, id: &str) -> Result<Option<Conversation>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, agent_id, title, created_at, updated_at FROM conversations WHERE id = ?1",
        )?;
        let row = stmt
            .query_row(params![id], |r| {
                Ok(Conversation {
                    id: r.get(0)?,
                    agent_id: r.get(1)?,
                    title: r.get(2)?,
                    created_at: r.get(3)?,
                    updated_at: r.get(4)?,
                })
            })
            .optional()?;
        Ok(row)
    }

    pub fn list_conversations(&self, limit: u32) -> Result<Vec<Conversation>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, agent_id, title, created_at, updated_at FROM conversations ORDER BY updated_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |r| {
            Ok(Conversation {
                id: r.get(0)?,
                agent_id: r.get(1)?,
                title: r.get(2)?,
                created_at: r.get(3)?,
                updated_at: r.get(4)?,
            })
        })?;
        let mut convs = Vec::new();
        for row in rows {
            convs.push(row?);
        }
        Ok(convs)
    }

    pub fn delete_conversation(&self, id: &str) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "DELETE FROM messages WHERE conversation_id = ?1",
            params![id],
        )?;
        conn.execute(
            "DELETE FROM usage_records WHERE conversation_id = ?1",
            params![id],
        )?;
        conn.execute("DELETE FROM conversations WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn add_message(&self, msg: &MessageRecord) -> Result<()> {
        let conn = self.conn()?;
        let tool_calls_json = msg
            .tool_calls
            .as_ref()
            .map(|tc| serde_json::to_string(tc).unwrap_or_default());
        let usage_json = msg
            .usage
            .as_ref()
            .map(|u| serde_json::to_string(u).unwrap_or_default());

        conn.execute(
            r#"INSERT INTO messages (id, conversation_id, parent_id, role, content, tool_calls, tool_call_id, model, usage, created_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"#,
            params![
                msg.id,
                msg.conversation_id,
                msg.parent_id,
                msg.role,
                msg.content,
                tool_calls_json,
                msg.tool_call_id,
                msg.model,
                usage_json,
                msg.created_at,
            ],
        )?;

        conn.execute(
            "UPDATE conversations SET updated_at = ?1 WHERE id = ?2",
            params![msg.created_at, msg.conversation_id],
        )?;
        Ok(())
    }

    pub fn get_messages(&self, conversation_id: &str) -> Result<Vec<MessageRecord>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, conversation_id, parent_id, role, content, tool_calls, tool_call_id, model, usage, created_at
             FROM messages WHERE conversation_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![conversation_id], |r| {
            let tool_calls_str: Option<String> = r.get(5)?;
            let usage_str: Option<String> = r.get(8)?;
            Ok(MessageRecord {
                id: r.get(0)?,
                conversation_id: r.get(1)?,
                parent_id: r.get(2)?,
                role: r.get(3)?,
                content: r.get(4)?,
                tool_calls: tool_calls_str.and_then(|s| serde_json::from_str(&s).ok()),
                tool_call_id: r.get(6)?,
                model: r.get(7)?,
                usage: usage_str.and_then(|s| serde_json::from_str(&s).ok()),
                created_at: r.get(9)?,
            })
        })?;
        let mut msgs = Vec::new();
        for row in rows {
            msgs.push(row?);
        }
        Ok(msgs)
    }

    pub fn record_usage(&self, record: &UsageRecord) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            r#"INSERT INTO usage_records (conversation_id, agent_id, model, prompt_tokens, completion_tokens, total_tokens, duration_ms, timestamp)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"#,
            params![
                record.conversation_id,
                record.agent_id,
                record.model,
                record.prompt_tokens,
                record.completion_tokens,
                record.total_tokens,
                record.duration_ms,
                record.timestamp,
            ],
        )?;
        Ok(())
    }

    pub fn get_usage_stats(&self) -> Result<UsageStats> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT COUNT(*), COALESCE(SUM(total_tokens), 0), COALESCE(SUM(duration_ms), 0) FROM usage_records",
        )?;
        let (total_requests, total_tokens, total_duration_ms): (i64, i64, i64) =
            stmt.query_row([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;

        let mut stmt2 = conn.prepare(
            "SELECT model, COUNT(*), COALESCE(SUM(total_tokens), 0) FROM usage_records GROUP BY model",
        )?;
        let rows = stmt2.query_map([], |r| {
            Ok(ModelUsage {
                model: r.get::<_, Option<String>>(0)?.unwrap_or_default(),
                requests: r.get(1)?,
                tokens: r.get(2)?,
            })
        })?;
        let mut by_model = Vec::new();
        for row in rows {
            by_model.push(row?);
        }

        Ok(UsageStats {
            total_requests: total_requests as u64,
            total_tokens: total_tokens as u64,
            total_duration_ms: total_duration_ms as u64,
            by_model,
        })
    }

    pub fn set_memory(&self, key: &str, value: &str, namespace: &str) -> Result<()> {
        let now = Utc::now().timestamp();
        let conn = self.conn()?;
        conn.execute(
            "INSERT OR REPLACE INTO memory (key, value, namespace, updated_at) VALUES (?1, ?2, ?3, ?4)",
            params![key, value, namespace, now],
        )?;
        Ok(())
    }

    pub fn get_memory(&self, key: &str, namespace: &str) -> Result<Option<String>> {
        let conn = self.conn()?;
        let mut stmt =
            conn.prepare("SELECT value FROM memory WHERE key = ?1 AND namespace = ?2")?;
        let row = stmt
            .query_row(params![key, namespace], |r| r.get::<_, String>(0))
            .optional()?;
        Ok(row)
    }

    pub fn list_memory(&self, namespace: &str) -> Result<Vec<MemoryEntry>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT key, value, updated_at FROM memory WHERE namespace = ?1 ORDER BY key",
        )?;
        let rows = stmt.query_map(params![namespace], |r| {
            Ok(MemoryEntry {
                key: r.get(0)?,
                value: r.get(1)?,
                updated_at: r.get(2)?,
            })
        })?;
        let mut entries = Vec::new();
        for row in rows {
            entries.push(row?);
        }
        Ok(entries)
    }

    pub fn delete_memory(&self, key: &str, namespace: &str) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "DELETE FROM memory WHERE key = ?1 AND namespace = ?2",
            params![key, namespace],
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub agent_id: String,
    pub title: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRecord {
    pub id: String,
    pub conversation_id: String,
    pub parent_id: Option<String>,
    pub role: String,
    pub content: String,
    pub tool_calls: Option<Vec<axon_core::ToolCall>>,
    pub tool_call_id: Option<String>,
    pub model: Option<String>,
    pub usage: Option<TokenUsage>,
    pub created_at: i64,
}

impl MessageRecord {
    pub fn new(conversation_id: &str, parent_id: Option<&str>, role: &str, content: &str) -> Self {
        MessageRecord {
            id: Uuid::new_v4().to_string(),
            conversation_id: conversation_id.into(),
            parent_id: parent_id.map(String::from),
            role: role.into(),
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            model: None,
            usage: None,
            created_at: Utc::now().timestamp(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecord {
    pub conversation_id: Option<String>,
    pub agent_id: Option<String>,
    pub model: Option<String>,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub duration_ms: u64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    pub total_requests: u64,
    pub total_tokens: u64,
    pub total_duration_ms: u64,
    pub by_model: Vec<ModelUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsage {
    pub model: String,
    pub requests: i64,
    pub tokens: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub key: String,
    pub value: String,
    pub updated_at: i64,
}

trait OptionalRow {
    fn optional(self) -> rusqlite::Result<Option<Self::Item>>
    where
        Self: Sized;
}

impl<T> OptionalRow for rusqlite::Result<T> {
    fn optional(self) -> rusqlite::Result<Option<T>> {
        match self {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}
