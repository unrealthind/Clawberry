//! Auditable worker archives and selective, source-aware memory promotion.
use super::workers::MemoryCandidate;
use super::{AgentHost, AgentId};
use anyhow::{bail, Context, Result};
use rusqlite::{params, Transaction};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub(super) fn migrate(tx: &Transaction<'_>) -> Result<()> {
    tx.execute_batch("CREATE TABLE IF NOT EXISTS memory_candidates (
        id TEXT PRIMARY KEY, owner_agent TEXT NOT NULL REFERENCES agents(id),worker_id TEXT NOT NULL REFERENCES workers(id),
        candidate TEXT NOT NULL,status TEXT NOT NULL DEFAULT 'pending',memory_id TEXT,
        created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,reviewed_at TEXT
    ); CREATE INDEX IF NOT EXISTS candidates_owner ON memory_candidates(owner_agent,status);
    CREATE UNIQUE INDEX IF NOT EXISTS candidate_worker_content ON memory_candidates(worker_id,candidate);
    CREATE TABLE IF NOT EXISTS archived_artifacts (
        id TEXT PRIMARY KEY,owner_agent TEXT NOT NULL REFERENCES agents(id),worker_id TEXT NOT NULL REFERENCES workers(id),
        filename TEXT NOT NULL,checksum TEXT NOT NULL,size INTEGER NOT NULL,
        UNIQUE(worker_id,filename)
    );")?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateRecord {
    pub id: String,
    pub owner_agent: AgentId,
    pub worker_id: String,
    pub candidate: MemoryCandidate,
    pub status: String,
    pub memory_id: Option<String>,
}

impl AgentHost {
    pub(super) fn archive_worker(&self, owner: AgentId, id: &str, status: &str) -> Result<()> {
        let worker = self.worker(owner, id)?;
        let task = self.task(owner, &worker.task_id)?;
        Uuid::parse_str(id)?;
        Uuid::parse_str(&task.id)?;
        let root = self.record(owner).workspace.join("archive");
        super::private_directory(&root)?;
        let task_dir = root.join(format!("task_{}", task.id));
        super::private_directory(&task_dir)?;
        let target = task_dir.join(id);
        // Recovery can finish DB bookkeeping after an atomic directory rename.
        if target.exists() {
            super::private_directory(&target)?;
        } else {
            let staging = task_dir.join(format!(".{}-{}", id, Uuid::new_v4()));
            super::private_directory(&staging)?;
            let artifacts = staging.join("artifacts");
            super::private_directory(&artifacts)?;
            let messages = {
                let db = self.connection()?;
                let mut q=db.prepare("SELECT sender,recipient,message_type,body,created_at FROM worker_messages WHERE worker_id=?1 AND owner_agent=?2 ORDER BY sequence")?;
                let collected = q.query_map(params![id,owner.as_str()],|r|Ok(serde_json::json!({"from":r.get::<_,String>(0)?,"to":r.get::<_,String>(1)?,"type":r.get::<_,String>(2)?,"body":r.get::<_,String>(3)?,"created_at":r.get::<_,String>(4)?})))?.collect::<rusqlite::Result<Vec<_>>>()?;
                collected
            };
            let summary = worker
                .result
                .as_ref()
                .map_or("No successful result", |r| r.summary.as_str());
            std::fs::write(
                staging.join("summary.md"),
                format!(
                    "# {}\n\nWorker: {id}\nOwner: {}\nStatus: {status}\n\n{summary}\n\nError: {}\n",
                    task.title,
                    owner.as_str(),
                    worker.error.as_deref().unwrap_or("none")
                ),
            )?;
            let transcript = messages
                .iter()
                .map(serde_json::to_string)
                .collect::<serde_json::Result<Vec<_>>>()?
                .join("\n");
            std::fs::write(staging.join("transcript.jsonl"), format!("{transcript}\n"))?;
            std::fs::write(staging.join("tool_calls.jsonl"), "")?;
            std::fs::write(
                staging.join("shell.log"),
                "No shell capability was granted. No shell commands were executed.\n",
            )?;
            let usage: Vec<_> = self
                .usage(owner)?
                .into_iter()
                .filter(|u| u.worker_id.as_deref() == Some(id))
                .collect();
            std::fs::write(
                staging.join("costs.json"),
                serde_json::to_vec_pretty(&usage)?,
            )?;
            std::fs::write(
                staging.join("sources.json"),
                serde_json::to_vec_pretty(&worker.result.as_ref().map(|r| &r.sources))?,
            )?;
            std::fs::write(
                staging.join("memory_candidates.json"),
                serde_json::to_vec_pretty(&worker.result.as_ref().map(|r| &r.memory_candidates))?,
            )?;
            if let Some(result) = &worker.result {
                result.validate(&worker.spec)?;
                for artifact in &result.artifacts {
                    std::fs::write(artifacts.join(&artifact.filename), &artifact.content)?;
                }
            }
            std::fs::rename(&staging, &target)?;
        }
        let mut db = self.connection()?;
        let tx = db.transaction()?;
        if let Some(result) = &worker.result {
            for candidate in &result.memory_candidates {
                tx.execute("INSERT OR IGNORE INTO memory_candidates(id,owner_agent,worker_id,candidate) VALUES (?1,?2,?3,?4)",params![Uuid::new_v4().to_string(),owner.as_str(),id,serde_json::to_string(candidate)?])?;
            }
            for artifact in &result.artifacts {
                tx.execute("INSERT OR IGNORE INTO archived_artifacts(id,owner_agent,worker_id,filename,checksum,size) VALUES (?1,?2,?3,?4,?5,?6)",params![Uuid::new_v4().to_string(),owner.as_str(),id,artifact.filename,hex::encode(Sha256::digest(artifact.content.as_bytes())),i64::try_from(artifact.content.len())?])?;
            }
        }
        tx.execute("UPDATE workers SET status=?1,archive_path=?2,ended_at=CURRENT_TIMESTAMP WHERE id=?3 AND owner_agent=?4",params![status,target.to_string_lossy(),id,owner.as_str()])?;
        let task_status = if status == "completed" {
            "needs_manni"
        } else {
            "blocked"
        };
        if tx.execute("UPDATE tasks SET status=?1,updated_at=CURRENT_TIMESTAMP WHERE id=?2 AND owner_agent=?3 AND status NOT IN ('completed','cancelled')",params![task_status,task.id,owner.as_str()])? == 1 {
            tx.execute("INSERT INTO task_events(task_id,event,detail) VALUES (?1,'worker_archived',?2)",params![task.id,format!("{id}: {status}; task {task_status}")])?;
        }
        tx.execute("INSERT INTO audit_events(request_id,owner_agent,event,detail) VALUES (?1,?2,'worker_archived',?3)",params![id,owner.as_str(),status])?;
        tx.commit()?;
        Ok(())
    }

    pub fn archive_file(&self, owner: AgentId, worker: &str, filename: &str) -> Result<Vec<u8>> {
        let record = self.worker(owner, worker)?;
        let path = std::path::PathBuf::from(
            record
                .archive_path
                .context("Worker has no completed archive")?,
        );
        let allowed = [
            "summary.md",
            "transcript.jsonl",
            "tool_calls.jsonl",
            "shell.log",
            "sources.json",
            "costs.json",
            "memory_candidates.json",
        ];
        let file = if allowed.contains(&filename) {
            path.join(filename)
        } else {
            let result = record.result.context("No worker artifacts")?;
            if !result.artifacts.iter().any(|a| a.filename == filename) {
                bail!("Archive artifact not found");
            }
            path.join("artifacts").join(filename)
        };
        super::reject_link(&file)?;
        let canonical = file.canonicalize()?;
        let root = self
            .record(owner)
            .workspace
            .join("archive")
            .canonicalize()?;
        if !canonical.starts_with(root) {
            bail!("Archive path escaped owner scope");
        }
        Ok(std::fs::read(canonical)?)
    }

    pub fn candidates(&self, owner: AgentId) -> Result<Vec<CandidateRecord>> {
        let db = self.connection()?;
        let mut q=db.prepare("SELECT id,worker_id,candidate,status,memory_id FROM memory_candidates WHERE owner_agent=?1 ORDER BY created_at,id")?;
        let rows = q.query_map([owner.as_str()], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, Option<String>>(4)?,
            ))
        })?;
        rows.map(|row| {
            let r = row?;
            Ok(CandidateRecord {
                id: r.0,
                owner_agent: owner,
                worker_id: r.1,
                candidate: serde_json::from_str(&r.2)?,
                status: r.3,
                memory_id: r.4,
            })
        })
        .collect()
    }

    pub fn reject_candidate(&self, owner: AgentId, id: &str) -> Result<()> {
        let changed=self.connection()?.execute("UPDATE memory_candidates SET status='rejected',reviewed_at=CURRENT_TIMESTAMP WHERE id=?1 AND owner_agent=?2 AND status='pending'",params![id,owner.as_str()])?;
        if changed != 1 {
            bail!("Pending candidate not found in selected domain");
        }
        self.audit(
            id,
            owner,
            "memory_candidate_rejected",
            "Selected fact rejected",
        )
    }

    pub fn promote_candidate(&self, owner: AgentId, id: &str) -> Result<String> {
        let candidate = self
            .candidates(owner)?
            .into_iter()
            .find(|c| c.id == id && c.status == "pending")
            .context("Pending candidate not found in selected domain")?;
        let fact = &candidate.candidate;
        let source = serde_json::to_string(
            &serde_json::json!({"worker_id":candidate.worker_id,"source_ref":fact.source_ref}),
        )?;
        super::knowledge::MemoryInput {
            content: fact.content.clone(),
            category: fact.category.clone(),
            confidence: fact.confidence,
            source_type: "subagent".into(),
            source_ref: source.clone(),
            sensitivity: fact.sensitivity.clone(),
            expires_at: None,
        }
        .validate()?;
        if fact.sensitivity != "ordinary" {
            bail!("Sensitive candidates are excluded from this promotion path");
        }
        let memory = self.record(owner).memory_db.clone();
        super::reject_link(&memory)?;
        let memory_id = Uuid::new_v4().to_string();
        let mut db = self.connection()?;
        db.execute(
            "ATTACH DATABASE ?1 AS private_memory",
            [memory.to_string_lossy().as_ref()],
        )?;
        let result = (|| -> Result<()> {
            let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            tx.execute_batch("CREATE TABLE IF NOT EXISTS private_memory.distilled_memories (
                id TEXT PRIMARY KEY,agent_owner TEXT NOT NULL,content TEXT NOT NULL,category TEXT NOT NULL,
                confidence REAL NOT NULL CHECK(confidence>=0 AND confidence<=1),source_type TEXT NOT NULL,source_ref TEXT NOT NULL,
                sensitivity TEXT NOT NULL,visibility TEXT NOT NULL DEFAULT 'private',
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,last_verified_at TEXT,expires_at TEXT
            );")?;
            if tx.execute("UPDATE memory_candidates SET status='promoted',memory_id=?1,reviewed_at=CURRENT_TIMESTAMP WHERE id=?2 AND owner_agent=?3 AND status='pending'",params![memory_id,id,owner.as_str()])?!=1{bail!("Candidate was already reviewed");}
            tx.execute("INSERT INTO private_memory.distilled_memories(id,agent_owner,content,category,confidence,source_type,source_ref,sensitivity) VALUES (?1,?2,?3,?4,?5,'subagent',?6,?7)",params![memory_id,owner.as_str(),fact.content,fact.category,fact.confidence,source,fact.sensitivity])?;
            tx.execute("INSERT INTO audit_events(request_id,owner_agent,event,detail) VALUES (?1,?2,'memory_promoted',?3)",params![id,owner.as_str(),memory_id])?;
            tx.commit()?;
            Ok(())
        })();
        let detached = db.execute_batch("DETACH DATABASE private_memory;");
        result?;
        detached?;
        Ok(memory_id)
    }
}
