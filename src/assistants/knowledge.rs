//! Source-aware semantic memory. Task transcripts are not semantic memory.
use super::{AgentHost, AgentId};
use anyhow::{bail, Result};
use rusqlite::{params, Connection, TransactionBehavior};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryInput {
    pub content: String,
    pub category: String,
    pub confidence: f64,
    pub source_type: String,
    pub source_ref: String,
    pub sensitivity: String,
    pub expires_at: Option<String>,
}
impl MemoryInput {
    pub(super) fn validate(&self) -> Result<()> {
        super::work::text(&self.content, 8_000, "Memory content")?;
        super::work::text(&self.category, 80, "Memory category")?;
        super::work::text(&self.source_ref, 2_000, "Memory provenance")?;
        if !self.confidence.is_finite()
            || !(0.0..=1.0).contains(&self.confidence)
            || !["user", "file", "web", "api", "subagent"].contains(&self.source_type.as_str())
            || !["ordinary", "sensitive"].contains(&self.sensitivity.as_str())
        {
            bail!("Invalid confidence, source type, or sensitivity");
        }
        if ["credential", "secret", "password", "token"]
            .iter()
            .any(|s| self.category.to_lowercase().contains(s))
        {
            bail!("Credentials belong in the secret broker, not semantic memory");
        }
        if self.sensitivity == "ordinary"
            && [
                "health",
                "medical",
                "financial",
                "finance",
                "identity",
                "legal",
            ]
            .iter()
            .any(|s| self.category.to_lowercase().contains(s))
        {
            bail!("Sensitive categories must be explicitly marked sensitive");
        }
        if let Some(expiry) = &self.expires_at {
            chrono::DateTime::parse_from_rfc3339(expiry)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryRecord {
    pub id: String,
    pub agent_owner: AgentId,
    #[serde(flatten)]
    pub fact: MemoryInput,
    pub visibility: String,
    pub created_at: String,
    pub last_verified_at: Option<String>,
    pub stale: bool,
    pub review_hash: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryFilter {
    #[serde(default)]
    pub query: String,
    pub source_type: Option<String>,
    pub minimum_confidence: Option<f64>,
}

pub(super) fn initialize(path: &std::path::Path) -> Result<()> {
    super::reject_link(path)?;
    let db = Connection::open(path)?;
    db.execute_batch("CREATE TABLE IF NOT EXISTS distilled_memories (
        id TEXT PRIMARY KEY,agent_owner TEXT NOT NULL,content TEXT NOT NULL,category TEXT NOT NULL,
        confidence REAL NOT NULL CHECK(confidence>=0 AND confidence<=1),source_type TEXT NOT NULL,source_ref TEXT NOT NULL,
        sensitivity TEXT NOT NULL,visibility TEXT NOT NULL DEFAULT 'private',
        created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,last_verified_at TEXT,expires_at TEXT,
        stale INTEGER NOT NULL DEFAULT 0 CHECK(stale IN (0,1)));")?;
    let has_stale = db
        .prepare("PRAGMA table_info(distilled_memories)")?
        .query_map([], |r| r.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?
        .iter()
        .any(|name| name == "stale");
    if !has_stale {
        db.execute_batch("ALTER TABLE distilled_memories ADD COLUMN stale INTEGER NOT NULL DEFAULT 0 CHECK(stale IN (0,1));")?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

impl AgentHost {
    pub fn review_expired_memories(&self, owner: AgentId) -> Result<usize> {
        self.memory_transaction(owner, |tx| {
            tx.execute("INSERT INTO audit_events(request_id,owner_agent,event,detail) SELECT id,agent_owner,'memory_expired','Explicit expiry reached; retained for human review' FROM knowledge.distilled_memories WHERE agent_owner=?1 AND stale=0 AND expires_at IS NOT NULL AND (julianday(expires_at) IS NULL OR julianday(expires_at)<=julianday('now'))", [owner.as_str()])?;
            Ok(tx.execute("UPDATE knowledge.distilled_memories SET stale=1 WHERE agent_owner=?1 AND stale=0 AND expires_at IS NOT NULL AND (julianday(expires_at) IS NULL OR julianday(expires_at)<=julianday('now'))", [owner.as_str()])?)
        })
    }

    /// Domain is selected by the trusted caller, never by a model-supplied path.
    fn memory_transaction<T>(
        &self,
        owner: AgentId,
        f: impl FnOnce(&rusqlite::Transaction<'_>) -> Result<T>,
    ) -> Result<T> {
        let path = &self.record(owner).memory_db;
        super::reject_link(path)?;
        let mut db = self.connection()?;
        db.execute(
            "ATTACH DATABASE ?1 AS knowledge",
            [path.to_string_lossy().as_ref()],
        )?;
        let result: Result<T> = (|| {
            let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let result = f(&tx)?;
            tx.commit()?;
            Ok(result)
        })();
        let detached = db.execute_batch("DETACH DATABASE knowledge;");
        let result = result?;
        detached?;
        Ok(result)
    }

    pub fn memories(&self, owner: AgentId, query: &str) -> Result<Vec<MemoryRecord>> {
        Ok(self
            .memory_page(
                owner,
                &MemoryFilter {
                    query: query.into(),
                    ..Default::default()
                },
                &super::pages::PageRequest {
                    limit: Some(200),
                    before: None,
                },
            )?
            .items)
    }

    pub fn memory_page(
        &self,
        owner: AgentId,
        filter: &MemoryFilter,
        page: &super::pages::PageRequest,
    ) -> Result<super::pages::Page<MemoryRecord>> {
        let (before, limit) = page.bounds()?;
        let minimum = filter.minimum_confidence.unwrap_or(0.0);
        if filter.query.len() > 500
            || !minimum.is_finite()
            || !(0.0..=1.0).contains(&minimum)
            || filter
                .source_type
                .as_ref()
                .is_some_and(|v| !["user", "file", "web", "api", "subagent"].contains(&v.as_str()))
        {
            bail!("Invalid memory search filter");
        }
        self.memory_transaction(owner, |tx| {
            let mut statement = tx.prepare("SELECT id,content,category,confidence,source_type,source_ref,sensitivity,expires_at,visibility,created_at,last_verified_at,stale,rowid FROM knowledge.distilled_memories WHERE agent_owner=?1 AND (?2='' OR instr(lower(content),lower(?2))>0 OR instr(lower(category),lower(?2))>0 OR instr(lower(source_ref),lower(?2))>0) AND (?3 IS NULL OR source_type=?3) AND confidence>=?4 AND rowid<?5 ORDER BY rowid DESC LIMIT ?6")?;
            let records = statement.query_map(params![owner.as_str(), filter.query, filter.source_type, minimum, before, limit + 1], |row| Ok((row.get::<_,i64>(12)?, MemoryRecord {
                id:row.get(0)?,agent_owner:owner,
                fact: MemoryInput {content:row.get(1)?,category:row.get(2)?,confidence:row.get(3)?,source_type:row.get(4)?,source_ref:row.get(5)?,sensitivity:row.get(6)?,expires_at:row.get(7)?},
                visibility:row.get(8)?,created_at:row.get(9)?,last_verified_at:row.get(10)?,stale:row.get(11)?,review_hash:String::new()
            })))?.collect::<rusqlite::Result<Vec<_>>>()?;
            let records = records.into_iter().map(|(cursor, mut record)| {record.review_hash = record.binding()?; Ok((cursor, record))}).collect::<Result<Vec<_>>>()?;
            Ok(super::pages::Page::from_rows(records, limit))
        })
    }

    /// Authenticated User management path; not granted as a model operation.
    pub fn save_memory(
        &self,
        owner: AgentId,
        id: Option<&str>,
        fact: &MemoryInput,
    ) -> Result<String> {
        fact.validate()?;
        let memory_id = id.map_or_else(|| uuid::Uuid::new_v4().to_string(), str::to_owned);
        self.memory_transaction(owner, |tx| {
            if id.is_some() {
                if tx.execute("UPDATE knowledge.distilled_memories SET content=?1,category=?2,confidence=?3,source_type=?4,source_ref=?5,sensitivity=?6,expires_at=?7,last_verified_at=NULL,stale=0 WHERE id=?8 AND agent_owner=?9",params![fact.content,fact.category,fact.confidence,fact.source_type,fact.source_ref,fact.sensitivity,fact.expires_at,memory_id,owner.as_str()])? != 1 { bail!("Memory not found in selected domain"); }
            } else {
                tx.execute("INSERT INTO knowledge.distilled_memories(id,agent_owner,content,category,confidence,source_type,source_ref,sensitivity,expires_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",params![memory_id,owner.as_str(),fact.content,fact.category,fact.confidence,fact.source_type,fact.source_ref,fact.sensitivity,fact.expires_at])?;
            }
            tx.execute("INSERT INTO audit_events(request_id,owner_agent,event,detail) VALUES(?1,?2,?3,?1)",params![memory_id,owner.as_str(),if id.is_some() {"memory_edited"} else {"memory_created"}])?;
            Ok(memory_id.clone())
        })
    }

    pub fn manage_memory(&self, owner: AgentId, id: &str, operation: &str) -> Result<()> {
        let (sql, event) = match operation {
            "delete" => ("DELETE FROM knowledge.distilled_memories WHERE id=?1 AND agent_owner=?2", "memory_deleted"),
            "verify" => ("UPDATE knowledge.distilled_memories SET last_verified_at=CURRENT_TIMESTAMP,stale=0 WHERE id=?1 AND agent_owner=?2 AND (expires_at IS NULL OR julianday(expires_at)>julianday('now'))", "memory_verified"),
            "stale" => ("UPDATE knowledge.distilled_memories SET stale=1 WHERE id=?1 AND agent_owner=?2", "memory_marked_stale"),
            _ => bail!("Unknown memory management operation"),
        };
        self.memory_transaction(owner, |tx| {
            if tx.execute(sql, params![id, owner.as_str()])? != 1 {
                bail!("Memory not found in selected domain");
            }
            tx.execute(
                "INSERT INTO audit_events(request_id,owner_agent,event,detail) VALUES(?1,?2,?3,?1)",
                params![id, owner.as_str(), event],
            )?;
            Ok(())
        })
    }

    /// Only ordinary, current private facts enter their owner's prompt. Shared
    /// memory is deliberately not injected across the permanent-agent boundary.
    pub(super) fn relevant_memories(
        &self,
        owner: AgentId,
        message: &str,
    ) -> Result<Vec<MemoryRecord>> {
        self.review_expired_memories(owner)?;
        let now = chrono::Utc::now();
        let words: Vec<_> = message
            .split_whitespace()
            .filter(|w| w.len() >= 4)
            .take(20)
            .map(str::to_lowercase)
            .collect();
        // Search each relevant term over the complete domain, rather than
        // filtering a truncated list of the newest 200 facts in application code.
        let mut selected = Vec::new();
        for word in words {
            let mut cursor = None;
            loop {
                let page = self.memory_page(
                    owner,
                    &MemoryFilter {
                        query: word.clone(),
                        minimum_confidence: Some(0.5),
                        source_type: None,
                    },
                    &super::pages::PageRequest {
                        before: cursor,
                        limit: Some(200),
                    },
                )?;
                for record in page.items {
                    if record.fact.sensitivity == "ordinary"
                        && !record.stale
                        && record.fact.expires_at.as_ref().is_none_or(|date| {
                            chrono::DateTime::parse_from_rfc3339(date)
                                .is_ok_and(|expiry| expiry > now)
                        })
                        && !selected.iter().any(|r: &MemoryRecord| r.id == record.id)
                    {
                        selected.push(record);
                        if selected.len() == 6 {
                            return Ok(selected);
                        }
                    }
                }
                cursor = page.next_cursor;
                if cursor.is_none() {
                    break;
                }
            }
        }
        Ok(selected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fact() -> MemoryInput {
        MemoryInput {
            content: "Project Orchard uses Rust".into(),
            category: "project".into(),
            confidence: 0.8,
            source_type: "user".into(),
            source_ref: "message:1".into(),
            sensitivity: "ordinary".into(),
            expires_at: None,
        }
    }
    #[test]
    fn expiry_review_is_owned_idempotent_and_never_deletes_facts() {
        let directory = tempfile::TempDir::new().unwrap();
        let host = AgentHost::open(directory.path()).unwrap();
        let expired = MemoryInput {
            expires_at: Some("2020-01-01T00:00:00Z".into()),
            ..fact()
        };
        let id = host.save_memory(AgentId::Janet, None, &expired).unwrap();
        host.save_memory(AgentId::Harley, None, &expired).unwrap();
        assert_eq!(host.review_expired_memories(AgentId::Janet).unwrap(), 1);
        assert_eq!(host.review_expired_memories(AgentId::Janet).unwrap(), 0);
        assert!(host.manage_memory(AgentId::Janet, &id, "verify").is_err());
        assert!(host.memories(AgentId::Janet, "").unwrap()[0].stale);
        assert!(!host.memories(AgentId::Harley, "").unwrap()[0].stale);
        assert!(host
            .relevant_memories(AgentId::Janet, "Orchard")
            .unwrap()
            .is_empty());
        let events: i64 = host
            .connection()
            .unwrap()
            .query_row(
                "SELECT count(*) FROM audit_events WHERE request_id=?1 AND event='memory_expired'",
                [&id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(events, 1);
        drop(host);
        let host = AgentHost::open(directory.path()).unwrap();
        assert!(host.memories(AgentId::Janet, "").unwrap()[0].stale);
        assert_eq!(host.review_expired_memories(AgentId::Harley).unwrap(), 1);
        assert_eq!(host.memories(AgentId::Harley, "").unwrap().len(), 1);
    }

    #[test]
    fn management_is_owned_durable_and_audited() {
        let d = tempfile::TempDir::new().unwrap();
        let host = AgentHost::open(d.path()).unwrap();
        let id = host.save_memory(AgentId::Janet, None, &fact()).unwrap();
        assert!(host.memories(AgentId::Harley, "").unwrap().is_empty());
        assert!(host.manage_memory(AgentId::Harley, &id, "delete").is_err());
        let mut edited = fact();
        edited.confidence = 0.9;
        host.save_memory(AgentId::Janet, Some(&id), &edited)
            .unwrap();
        host.manage_memory(AgentId::Janet, &id, "verify").unwrap();
        assert!(host.memories(AgentId::Janet, "Orchard").unwrap()[0]
            .last_verified_at
            .is_some());
        host.manage_memory(AgentId::Janet, &id, "stale").unwrap();
        assert!(host
            .relevant_memories(AgentId::Janet, "Orchard")
            .unwrap()
            .is_empty());
        drop(host);
        let host = AgentHost::open(d.path()).unwrap();
        assert!(host.memories(AgentId::Janet, "").unwrap()[0].stale);
        host.manage_memory(AgentId::Janet, &id, "delete").unwrap();
        assert!(host.memories(AgentId::Janet, "").unwrap().is_empty());
        let events: i64 = host
            .connection()
            .unwrap()
            .query_row(
                "SELECT count(*) FROM audit_events WHERE request_id=?1",
                [id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(events, 5);
    }
    #[test]
    fn invalid_and_sensitive_memory_never_auto_enters_prompt() {
        let d = tempfile::TempDir::new().unwrap();
        let host = AgentHost::open(d.path()).unwrap();
        let mut input = fact();
        input.confidence = f64::NAN;
        assert!(host.save_memory(AgentId::Janet, None, &input).is_err());
        input = fact();
        input.category = "password".into();
        assert!(host.save_memory(AgentId::Janet, None, &input).is_err());
        input = fact();
        input.sensitivity = "sensitive".into();
        host.save_memory(AgentId::Janet, None, &input).unwrap();
        assert!(host
            .relevant_memories(AgentId::Janet, "Orchard")
            .unwrap()
            .is_empty());
    }
}
