//! Explicit human promotion into a separate shared domain. No peer-private reads.
use super::{
    knowledge::{MemoryInput, MemoryRecord},
    pages::{Page, PageRequest},
    AgentHost, AgentId,
};
use anyhow::{bail, Result};
use rusqlite::{params, Connection, TransactionBehavior};
use sha2::{Digest, Sha256};

pub(super) fn initialize(path: &std::path::Path) -> Result<()> {
    super::knowledge::initialize(path)?;
    Connection::open(path)?.execute_batch("CREATE TABLE IF NOT EXISTS shared_provenance(shared_id TEXT PRIMARY KEY REFERENCES distilled_memories(id) ON DELETE CASCADE,private_id TEXT NOT NULL,owner_agent TEXT NOT NULL,reviewed_hash TEXT NOT NULL,UNIQUE(owner_agent,private_id));")?;
    Ok(())
}
impl MemoryRecord {
    pub fn binding(&self) -> Result<String> {
        Ok(hex::encode(Sha256::digest(serde_json::to_vec(
            &serde_json::json!({"id":self.id,"owner":self.agent_owner,"fact":self.fact,"visibility":self.visibility,"stale":self.stale,"verified":self.last_verified_at}),
        )?)))
    }
}
impl AgentHost {
    pub fn update_shared_memory(
        &self,
        owner: AgentId,
        id: &str,
        reviewed_hash: &str,
        fact: Option<&MemoryInput>,
        operation: &str,
    ) -> Result<()> {
        if let Some(fact) = fact {
            fact.validate()?;
            if fact.sensitivity != "ordinary" {
                bail!("Shared memory must remain ordinary");
            }
        }
        self.shared_transaction(None, |tx| {
            let record = tx.query_row("SELECT content,category,confidence,source_type,source_ref,sensitivity,expires_at,created_at,last_verified_at,stale FROM shared.distilled_memories WHERE id=?1 AND agent_owner=?2", params![id,owner.as_str()], |row| Ok(MemoryRecord {
                id:id.into(), agent_owner:owner,
                fact:MemoryInput { content:row.get(0)?, category:row.get(1)?, confidence:row.get(2)?, source_type:row.get(3)?, source_ref:row.get(4)?, sensitivity:row.get(5)?, expires_at:row.get(6)? },
                visibility:"shared".into(), created_at:row.get(7)?, last_verified_at:row.get(8)?, stale:row.get(9)?, review_hash:String::new(),
            }))?;
            if record.binding()? != reviewed_hash {
                bail!("Shared memory changed; review its current content before updating");
            }
            let event = match (operation, fact) {
                ("save", Some(fact)) => {
                    tx.execute("UPDATE shared.distilled_memories SET content=?1,category=?2,confidence=?3,source_type=?4,source_ref=?5,sensitivity=?6,expires_at=?7,last_verified_at=NULL,stale=0 WHERE id=?8 AND agent_owner=?9",params![fact.content,fact.category,fact.confidence,fact.source_type,fact.source_ref,fact.sensitivity,fact.expires_at,id,owner.as_str()])?;
                    "shared_memory_edited"
                }
                ("verify", None) => {
                    if record.fact.expires_at.as_ref().is_some_and(|expiry| chrono::DateTime::parse_from_rfc3339(expiry).map_or(true, |date|date<=chrono::Utc::now())) {
                        bail!("Expired shared memory requires an explicit content/expiry edit before verification");
                    }
                    tx.execute("UPDATE shared.distilled_memories SET last_verified_at=CURRENT_TIMESTAMP,stale=0 WHERE id=?1 AND agent_owner=?2",params![id,owner.as_str()])?;
                    "shared_memory_verified"
                }
                ("stale", None) => {
                    tx.execute("UPDATE shared.distilled_memories SET stale=1 WHERE id=?1 AND agent_owner=?2",params![id,owner.as_str()])?;
                    "shared_memory_marked_stale"
                }
                _ => bail!("Invalid shared-memory management operation"),
            };
            tx.execute("INSERT INTO audit_events(request_id,owner_agent,event,detail) VALUES(?1,?2,?3,?4)",params![id,owner.as_str(),event,reviewed_hash])?;
            Ok(())
        })
    }

    fn shared_transaction<T>(
        &self,
        source: Option<&std::path::Path>,
        f: impl FnOnce(&rusqlite::Transaction<'_>) -> Result<T>,
    ) -> Result<T> {
        let path = self.root.join("data/shared.db");
        super::reject_link(&path)?;
        if let Some(source) = source {
            super::reject_link(source)?;
        }
        let mut db = self.connection()?;
        db.execute(
            "ATTACH DATABASE ?1 AS shared",
            [path.to_string_lossy().as_ref()],
        )?;
        if let Some(source) = source {
            if let Err(error) = db.execute(
                "ATTACH DATABASE ?1 AS private_source",
                [source.to_string_lossy().as_ref()],
            ) {
                db.execute_batch("DETACH DATABASE shared;")?;
                return Err(error.into());
            }
        }
        let result: Result<T> = (|| {
            let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let value = f(&tx)?;
            tx.commit()?;
            Ok(value)
        })();
        let detached = db.execute_batch(if source.is_some() {
            "DETACH DATABASE private_source; DETACH DATABASE shared;"
        } else {
            "DETACH DATABASE shared;"
        });
        let value = result?;
        detached?;
        Ok(value)
    }
    /// Exact private content and metadata must still match what User reviewed.
    /// Called only from the authenticated human API, never an agent capability.
    pub fn share_memory(&self, owner: AgentId, id: &str, reviewed_hash: &str) -> Result<String> {
        // Both the source and shared copy are attached to the same system
        // transaction so an edit cannot race the exact-content review check.
        let private = self.record(owner).memory_db.clone();
        self.shared_transaction(Some(&private), |tx| {
            let result=(|| -> Result<String> {
                let memory=tx.query_row("SELECT id,content,category,confidence,source_type,source_ref,sensitivity,expires_at,visibility,created_at,last_verified_at,stale FROM private_source.distilled_memories WHERE id=?1 AND agent_owner=?2",params![id,owner.as_str()],|r|Ok(MemoryRecord {id:r.get(0)?,agent_owner:owner,fact:MemoryInput {content:r.get(1)?,category:r.get(2)?,confidence:r.get(3)?,source_type:r.get(4)?,source_ref:r.get(5)?,sensitivity:r.get(6)?,expires_at:r.get(7)?},visibility:r.get(8)?,created_at:r.get(9)?,last_verified_at:r.get(10)?,stale:r.get(11)?,review_hash:String::new()}))?;
                if memory.binding()?!=reviewed_hash || memory.stale || memory.fact.sensitivity!="ordinary" {bail!("Memory changed, is stale, or is sensitive; review an ordinary current fact before sharing");}
                if memory.fact.expires_at.as_ref().is_some_and(|v|chrono::DateTime::parse_from_rfc3339(v).is_ok_and(|date|date<=chrono::Utc::now())) {bail!("Expired facts cannot be shared");}
                let shared_id=uuid::Uuid::new_v4().to_string();
                let fact=memory.fact;
                tx.execute("INSERT INTO shared.distilled_memories(id,agent_owner,content,category,confidence,source_type,source_ref,sensitivity,visibility,expires_at,last_verified_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,'shared',?9,?10)",params![shared_id,owner.as_str(),fact.content,fact.category,fact.confidence,fact.source_type,fact.source_ref,fact.sensitivity,fact.expires_at,memory.last_verified_at])?;
                tx.execute("INSERT INTO shared.shared_provenance(shared_id,private_id,owner_agent,reviewed_hash) VALUES(?1,?2,?3,?4)",params![shared_id,id,owner.as_str(),reviewed_hash])?;
                tx.execute("INSERT INTO audit_events(request_id,owner_agent,event,detail) VALUES(?1,?2,'memory_explicitly_shared',?3)",params![shared_id,owner.as_str(),id])?;
                Ok(shared_id)
            })();
            // SQLite won't detach a database referenced by an active transaction.
            // Detach after commit/rollback in the outer helper instead.
            result
        })
    }
    pub fn shared_page(&self, page: &PageRequest) -> Result<Page<MemoryRecord>> {
        let (before, limit) = page.bounds()?;
        self.shared_transaction(None, |tx| {
            let mut q=tx.prepare("SELECT rowid,id,agent_owner,content,category,confidence,source_type,source_ref,sensitivity,expires_at,created_at,last_verified_at,stale FROM shared.distilled_memories WHERE rowid<?1 ORDER BY rowid DESC LIMIT ?2")?;
            let rows=q.query_map(params![before,limit+1],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(2)?,MemoryRecord {id:r.get(1)?,agent_owner:AgentId::Janet,fact:MemoryInput {content:r.get(3)?,category:r.get(4)?,confidence:r.get(5)?,source_type:r.get(6)?,source_ref:r.get(7)?,sensitivity:r.get(8)?,expires_at:r.get(9)?},visibility:"shared".into(),created_at:r.get(10)?,last_verified_at:r.get(11)?,stale:r.get(12)?,review_hash:String::new()})))?.collect::<rusqlite::Result<Vec<_>>>()?;
            let rows=rows.into_iter().map(|(cursor,owner,mut record)| {record.agent_owner=AgentId::parse(&owner)?;record.review_hash=record.binding()?;Ok((cursor,record))}).collect::<Result<Vec<_>>>()?;
            Ok(Page::from_rows(rows,limit))
        })
    }
    pub fn unshare_memory(&self, owner: AgentId, id: &str) -> Result<()> {
        self.shared_transaction(None, |tx| {
            if tx.execute("DELETE FROM shared.distilled_memories WHERE id=?1 AND agent_owner=?2",params![id,owner.as_str()])?!=1 {bail!("Shared memory not found in selected origin domain");}
            tx.execute("DELETE FROM shared.shared_provenance WHERE shared_id=?1 AND owner_agent=?2",params![id,owner.as_str()])?;
            tx.execute("INSERT INTO audit_events(request_id,owner_agent,event,detail) VALUES(?1,?2,'memory_unshared',?1)",params![id,owner.as_str()])?;
            Ok(())
        })
    }
}

impl AgentHost {
    pub(super) fn relevant_shared(&self, message: &str) -> Result<Vec<MemoryRecord>> {
        let words = message
            .split_whitespace()
            .filter(|w| w.len() >= 4)
            .take(20)
            .map(str::to_lowercase)
            .collect::<Vec<_>>();
        let mut before = None;
        let mut selected = Vec::new();
        if words.is_empty() {
            return Ok(selected);
        }
        loop {
            let page = self.shared_page(&PageRequest {
                before,
                limit: Some(200),
            })?;
            for record in page.items {
                if !record.stale
                    && record.fact.sensitivity == "ordinary"
                    && record.fact.confidence >= 0.5
                    && record.fact.expires_at.as_ref().is_none_or(|d| {
                        chrono::DateTime::parse_from_rfc3339(d)
                            .is_ok_and(|date| date > chrono::Utc::now())
                    })
                    && words
                        .iter()
                        .any(|w| record.fact.content.to_lowercase().contains(w))
                {
                    selected.push(record);
                    if selected.len() == 6 {
                        return Ok(selected);
                    }
                }
            }
            before = page.next_cursor;
            if before.is_none() {
                return Ok(selected);
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fact() -> MemoryInput {
        MemoryInput {
            content: "Orchard delivery is Friday".into(),
            category: "project".into(),
            confidence: 0.9,
            source_type: "user".into(),
            source_ref: "message:reviewed".into(),
            sensitivity: "ordinary".into(),
            expires_at: None,
        }
    }
    #[test]
    #[allow(clippy::too_many_lines)]
    fn shared_management_is_exact_owned_durable_and_separate_from_private_source() {
        let directory = tempfile::TempDir::new().unwrap();
        let host = AgentHost::open(directory.path()).unwrap();
        let private_id = host.save_memory(AgentId::Janet, None, &fact()).unwrap();
        let source = host.memories(AgentId::Janet, "").unwrap().remove(0);
        let shared_id = host
            .share_memory(AgentId::Janet, &private_id, &source.review_hash)
            .unwrap();
        let record = host
            .shared_page(&PageRequest::default())
            .unwrap()
            .items
            .remove(0);
        let edited = MemoryInput {
            content: "Orchard delivery is Monday".into(),
            ..fact()
        };
        assert!(host
            .update_shared_memory(
                AgentId::Harley,
                &shared_id,
                &record.review_hash,
                Some(&edited),
                "save"
            )
            .is_err());
        assert!(host
            .update_shared_memory(AgentId::Janet, &shared_id, "wrong", Some(&edited), "save")
            .is_err());
        assert!(host
            .update_shared_memory(
                AgentId::Janet,
                &shared_id,
                &record.review_hash,
                Some(&MemoryInput {
                    sensitivity: "sensitive".into(),
                    ..edited.clone()
                }),
                "save"
            )
            .is_err());
        host.update_shared_memory(
            AgentId::Janet,
            &shared_id,
            &record.review_hash,
            Some(&edited),
            "save",
        )
        .unwrap();
        assert!(host
            .update_shared_memory(
                AgentId::Janet,
                &shared_id,
                &record.review_hash,
                None,
                "verify"
            )
            .is_err());
        assert_eq!(
            host.memories(AgentId::Janet, "").unwrap()[0].fact.content,
            fact().content
        );
        let current = host
            .shared_page(&PageRequest::default())
            .unwrap()
            .items
            .remove(0);
        host.update_shared_memory(
            AgentId::Janet,
            &shared_id,
            &current.review_hash,
            None,
            "stale",
        )
        .unwrap();
        assert!(host.relevant_shared("Orchard").unwrap().is_empty());
        drop(host);
        let host = AgentHost::open(directory.path()).unwrap();
        let current = host
            .shared_page(&PageRequest::default())
            .unwrap()
            .items
            .remove(0);
        assert!(current.stale);
        host.update_shared_memory(
            AgentId::Janet,
            &shared_id,
            &current.review_hash,
            None,
            "verify",
        )
        .unwrap();
        let current = host
            .shared_page(&PageRequest::default())
            .unwrap()
            .items
            .remove(0);
        assert!(current.last_verified_at.is_some());
        assert!(!current.stale);
        assert_eq!(
            host.relevant_shared("Orchard").unwrap()[0].fact.content,
            edited.content
        );
        let expired = MemoryInput {
            expires_at: Some("2020-01-01T00:00:00Z".into()),
            ..edited
        };
        host.update_shared_memory(
            AgentId::Janet,
            &shared_id,
            &current.review_hash,
            Some(&expired),
            "save",
        )
        .unwrap();
        let current = host
            .shared_page(&PageRequest::default())
            .unwrap()
            .items
            .remove(0);
        assert!(host
            .update_shared_memory(
                AgentId::Janet,
                &shared_id,
                &current.review_hash,
                None,
                "verify"
            )
            .is_err());
        assert!(host.relevant_shared("Orchard").unwrap().is_empty());
        host.unshare_memory(AgentId::Janet, &shared_id).unwrap();
        assert!(host
            .shared_page(&PageRequest::default())
            .unwrap()
            .items
            .is_empty());
    }

    #[test]
    fn exact_human_promotion_copies_only_selected_fact_and_can_be_revoked() {
        let d = tempfile::TempDir::new().unwrap();
        let host = AgentHost::open(d.path()).unwrap();
        let id = host.save_memory(AgentId::Janet, None, &fact()).unwrap();
        let record = host.memories(AgentId::Janet, "").unwrap().remove(0);
        assert!(host
            .share_memory(AgentId::Harley, &id, &record.review_hash)
            .is_err());
        assert!(host
            .share_memory(AgentId::Janet, &id, "wrong-hash")
            .is_err());
        let shared = host
            .share_memory(AgentId::Janet, &id, &record.review_hash)
            .unwrap();
        assert!(host
            .share_memory(AgentId::Janet, &id, &record.review_hash)
            .is_err());
        assert_eq!(
            host.shared_page(&PageRequest::default()).unwrap().items[0].visibility,
            "shared"
        );
        assert!(host.memories(AgentId::Harley, "").unwrap().is_empty());
        assert_eq!(host.relevant_shared("Orchard").unwrap().len(), 1);
        assert!(host.unshare_memory(AgentId::Harley, &shared).is_err());
        host.unshare_memory(AgentId::Janet, &shared).unwrap();
        assert!(host
            .shared_page(&PageRequest::default())
            .unwrap()
            .items
            .is_empty());
        assert_eq!(host.memories(AgentId::Janet, "").unwrap().len(), 1);
        host.save_memory(
            AgentId::Janet,
            Some(&id),
            &MemoryInput {
                content: "Changed fact".into(),
                ..fact()
            },
        )
        .unwrap();
        assert!(host
            .share_memory(AgentId::Janet, &id, &record.review_hash)
            .is_err());
    }
    #[test]
    fn sensitive_stale_and_expired_facts_are_never_promoted() {
        let d = tempfile::TempDir::new().unwrap();
        let host = AgentHost::open(d.path()).unwrap();
        for fact in [
            MemoryInput {
                sensitivity: "sensitive".into(),
                ..fact()
            },
            MemoryInput {
                expires_at: Some("2000-01-01T00:00:00Z".into()),
                ..fact()
            },
        ] {
            let id = host.save_memory(AgentId::Janet, None, &fact).unwrap();
            let record = host
                .memories(AgentId::Janet, "")
                .unwrap()
                .into_iter()
                .find(|r| r.id == id)
                .unwrap();
            assert!(host
                .share_memory(AgentId::Janet, &id, &record.review_hash)
                .is_err());
        }
        let id = host.save_memory(AgentId::Janet, None, &fact()).unwrap();
        host.manage_memory(AgentId::Janet, &id, "stale").unwrap();
        let record = host
            .memories(AgentId::Janet, "")
            .unwrap()
            .into_iter()
            .find(|r| r.id == id)
            .unwrap();
        assert!(host
            .share_memory(AgentId::Janet, &id, &record.review_hash)
            .is_err());
    }
}
