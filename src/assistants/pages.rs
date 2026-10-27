//! Bounded human-facing history pages, independent of model context limits.
use super::{AgentHost, AgentId, MessageEnvelope};
use anyhow::{bail, Result};
use rusqlite::params;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PageRequest {
    pub before: Option<i64>,
    pub limit: Option<u32>,
}
impl PageRequest {
    pub fn bounds(&self) -> Result<(i64, i64)> {
        let limit = self.limit.unwrap_or(50);
        if !(1..=200).contains(&limit) || self.before.is_some_and(|v| v <= 0) {
            bail!("Page limit must be 1–200 and cursor must be positive");
        }
        Ok((self.before.unwrap_or(i64::MAX), i64::from(limit)))
    }
}
#[derive(Debug, Serialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<i64>,
}
impl<T> Page<T> {
    pub fn from_rows(mut rows: Vec<(i64, T)>, limit: i64) -> Self {
        let limit = usize::try_from(limit).expect("validated positive page limit");
        let has_more = rows.len() > limit;
        rows.truncate(limit);
        let next_cursor = has_more.then(|| rows.last().expect("positive page size").0);
        Self {
            items: rows.into_iter().map(|(_, value)| value).collect(),
            next_cursor,
        }
    }
}
#[derive(Debug, Serialize)]
pub struct HistoryMessage {
    pub sequence: i64,
    pub role: String,
    pub content: String,
}
impl AgentHost {
    /// Bounded owner-scoped workspace rows; rowid cursors do not grant access.
    pub fn task_page(&self, owner: AgentId, page: &PageRequest) -> Result<Page<super::work::Task>> {
        let (before, limit) = page.bounds()?;
        let db = self.connection()?;
        let mut query = db.prepare("SELECT rowid,id FROM tasks WHERE owner_agent=?1 AND rowid<?2 ORDER BY rowid DESC LIMIT ?3")?;
        let ids = query
            .query_map(params![owner.as_str(), before, limit + 1], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(query);
        drop(db);
        let rows = ids
            .into_iter()
            .map(|(cursor, id)| self.task(owner, &id).map(|task| (cursor, task)))
            .collect::<Result<Vec<_>>>()?;
        Ok(Page::from_rows(rows, limit))
    }

    pub fn thread_page(
        &self,
        owner: AgentId,
        page: &PageRequest,
    ) -> Result<Page<super::work::FocusThread>> {
        let (before, limit) = page.bounds()?;
        let db = self.connection()?;
        let mut query = db.prepare("SELECT rowid,id FROM focus_threads WHERE owner_agent=?1 AND rowid<?2 ORDER BY rowid DESC LIMIT ?3")?;
        let ids = query
            .query_map(params![owner.as_str(), before, limit + 1], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(query);
        drop(db);
        let rows = ids
            .into_iter()
            .map(|(cursor, id)| self.thread(owner, &id).map(|thread| (cursor, thread)))
            .collect::<Result<Vec<_>>>()?;
        Ok(Page::from_rows(rows, limit))
    }

    /// Newest first. A cursor is a position only, never authorization.
    pub fn history_page(
        &self,
        owner: AgentId,
        thread: Option<&str>,
        page: &PageRequest,
    ) -> Result<Page<HistoryMessage>> {
        let (before, limit) = page.bounds()?;
        let session = if let Some(id) = thread {
            self.thread(owner, id)?.id
        } else {
            self.record(owner).main_session_id.clone()
        };
        let db = self.connection()?;
        let sql = if thread.is_some() {
            "SELECT sequence,role,envelope FROM thread_messages WHERE thread_id=?1 AND sequence<?2 ORDER BY sequence DESC LIMIT ?3"
        } else {
            "SELECT sequence,role,envelope FROM messages WHERE session_id=?1 AND sequence<?2 ORDER BY sequence DESC LIMIT ?3"
        };
        let mut query = db.prepare(sql)?;
        let rows = query.query_map(params![session, before, limit + 1], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?;
        let mut result = Vec::new();
        for row in rows {
            let (sequence, role, json) = row?;
            let envelope: MessageEnvelope = serde_json::from_str(&json)?;
            if envelope.owner_agent != owner || envelope.session_id != session {
                bail!("Conversation ownership mismatch");
            }
            result.push((
                sequence,
                HistoryMessage {
                    sequence,
                    role,
                    content: envelope.content,
                },
            ));
        }
        Ok(Page::from_rows(result, limit))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn workspace_pages_are_bounded_complete_and_owner_scoped() {
        let d = tempfile::TempDir::new().unwrap();
        let host = AgentHost::open(d.path()).unwrap();
        for i in 0..125 {
            host.create_task(AgentId::Janet, &format!("Task {i}"), "Evidence", None)
                .unwrap();
            host.create_thread(AgentId::Janet, &format!("Thread {i}"))
                .unwrap();
        }
        host.create_task(AgentId::Harley, "Private", "Personal", None)
            .unwrap();
        host.create_thread(AgentId::Harley, "Private").unwrap();
        for tasks in [true, false] {
            let mut before = None;
            let mut ids = std::collections::HashSet::new();
            loop {
                let request = PageRequest {
                    before,
                    limit: Some(17),
                };
                let (rows, next) = if tasks {
                    let page = host.task_page(AgentId::Janet, &request).unwrap();
                    (
                        page.items
                            .into_iter()
                            .map(|t| (t.id, t.title))
                            .collect::<Vec<_>>(),
                        page.next_cursor,
                    )
                } else {
                    let page = host.thread_page(AgentId::Janet, &request).unwrap();
                    (
                        page.items
                            .into_iter()
                            .map(|t| (t.id, t.title))
                            .collect::<Vec<_>>(),
                        page.next_cursor,
                    )
                };
                assert!(rows.len() <= 17);
                for (id, title) in rows {
                    assert_ne!(title, "Private");
                    assert!(ids.insert(id));
                }
                if next.is_none() {
                    break;
                }
                assert!(before.is_none_or(|cursor| next.unwrap() < cursor));
                before = next;
            }
            assert_eq!(ids.len(), 125);
        }
        assert_eq!(
            host.task_page(AgentId::Harley, &PageRequest::default())
                .unwrap()
                .items
                .len(),
            1
        );
        assert_eq!(
            host.thread_page(AgentId::Harley, &PageRequest::default())
                .unwrap()
                .items
                .len(),
            1
        );
        for limit in [0, 201] {
            assert!(host
                .task_page(
                    AgentId::Janet,
                    &PageRequest {
                        before: None,
                        limit: Some(limit)
                    }
                )
                .is_err());
            assert!(host
                .thread_page(
                    AgentId::Janet,
                    &PageRequest {
                        before: None,
                        limit: Some(limit)
                    }
                )
                .is_err());
        }
    }

    #[test]
    fn complete_history_is_paginated_without_expanding_model_context_or_peer_access() {
        let d = tempfile::TempDir::new().unwrap();
        let host = AgentHost::open(d.path()).unwrap();
        let session = host.record(AgentId::Janet).main_session_id.clone();
        for index in 0..125 {
            let envelope = MessageEnvelope {
                id: uuid::Uuid::new_v4().to_string(),
                from: "user".into(),
                to: "janet".into(),
                owner_agent: AgentId::Janet,
                session_id: session.clone(),
                message_type: "chat".into(),
                content: format!("message {index}"),
            };
            host.connection()
                .unwrap()
                .execute(
                    "INSERT INTO messages(id,session_id,role,envelope) VALUES(?1,?2,'user',?3)",
                    params![
                        envelope.id,
                        session,
                        serde_json::to_string(&envelope).unwrap()
                    ],
                )
                .unwrap();
        }
        let mut before = None;
        let mut messages = Vec::new();
        loop {
            let page = host
                .history_page(
                    AgentId::Janet,
                    None,
                    &PageRequest {
                        before,
                        limit: Some(40),
                    },
                )
                .unwrap();
            messages.extend(page.items);
            before = page.next_cursor;
            if before.is_none() {
                break;
            }
        }
        assert_eq!(messages.len(), 125);
        assert!(messages.windows(2).all(|p| p[0].sequence > p[1].sequence));
        assert_eq!(host.history(AgentId::Janet).unwrap().len(), 50);
        assert!(host
            .history_page(AgentId::Harley, None, &PageRequest::default())
            .unwrap()
            .items
            .is_empty());
        let thread = host.create_thread(AgentId::Janet, "Private").unwrap();
        assert!(host
            .history_page(AgentId::Harley, Some(&thread.id), &PageRequest::default())
            .is_err());
        assert!(host
            .history_page(
                AgentId::Janet,
                None,
                &PageRequest {
                    before: Some(-1),
                    limit: Some(201)
                }
            )
            .is_err());
    }
    #[test]
    fn memory_filters_and_retrieval_cover_records_older_than_first_page() {
        use crate::assistants::knowledge::{MemoryFilter, MemoryInput};
        let d = tempfile::TempDir::new().unwrap();
        let host = AgentHost::open(d.path()).unwrap();
        let fact = MemoryInput {
            content: "Orchard uses Rust".into(),
            category: "project".into(),
            confidence: 0.9,
            source_type: "user".into(),
            source_ref: "message:1".into(),
            sensitivity: "ordinary".into(),
            expires_at: None,
        };
        let oldest = host.save_memory(AgentId::Janet, None, &fact).unwrap();
        for i in 0..205 {
            host.save_memory(
                AgentId::Janet,
                None,
                &MemoryInput {
                    content: format!("Unrelated fact {i}"),
                    source_type: "file".into(),
                    ..fact.clone()
                },
            )
            .unwrap();
        }
        let filter = MemoryFilter::default();
        let first = host
            .memory_page(
                AgentId::Janet,
                &filter,
                &PageRequest {
                    limit: Some(200),
                    before: None,
                },
            )
            .unwrap();
        assert_eq!(first.items.len(), 200);
        let second = host
            .memory_page(
                AgentId::Janet,
                &filter,
                &PageRequest {
                    before: first.next_cursor,
                    limit: Some(200),
                },
            )
            .unwrap();
        assert_eq!(second.items.len(), 6);
        assert!(second.next_cursor.is_none());
        let user = host
            .memory_page(
                AgentId::Janet,
                &MemoryFilter {
                    source_type: Some("user".into()),
                    minimum_confidence: Some(0.8),
                    ..Default::default()
                },
                &PageRequest::default(),
            )
            .unwrap();
        assert_eq!(user.items.len(), 1);
        assert_eq!(user.items[0].id, oldest);
        assert_eq!(
            host.relevant_memories(AgentId::Janet, "What does Orchard use?")
                .unwrap()[0]
                .id,
            oldest
        );
        assert!(host
            .memory_page(
                AgentId::Harley,
                &filter,
                &PageRequest {
                    before: first.next_cursor,
                    limit: Some(200)
                }
            )
            .unwrap()
            .items
            .is_empty());
    }
}
