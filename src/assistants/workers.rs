//! Ephemeral, task-scoped model workers. No execution tools or secrets are inherited.
use super::models::{InferenceScope, Profile};
use super::work::{text, TaskStatus};
use super::{AgentHost, AgentId};
use crate::config::Config;
use crate::providers::ChatMessage;
use anyhow::{bail, Context, Result};
use rusqlite::{params, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use tokio::sync::watch;
use uuid::Uuid;

pub(super) fn migrate(tx: &Transaction<'_>) -> Result<()> {
    tx.execute_batch("CREATE TABLE IF NOT EXISTS workers (
        id TEXT PRIMARY KEY, owner_agent TEXT NOT NULL REFERENCES agents(id), task_id TEXT NOT NULL REFERENCES tasks(id),
        spec TEXT NOT NULL, status TEXT NOT NULL, result TEXT, error TEXT, archive_path TEXT,
        lease_expires_at INTEGER NOT NULL, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
        ended_at TEXT
    ); CREATE INDEX IF NOT EXISTS workers_task ON workers(task_id);
    CREATE TABLE IF NOT EXISTS worker_messages (
        sequence INTEGER PRIMARY KEY AUTOINCREMENT, worker_id TEXT NOT NULL REFERENCES workers(id),
        owner_agent TEXT NOT NULL REFERENCES agents(id), sender TEXT NOT NULL, recipient TEXT NOT NULL,
        message_type TEXT NOT NULL, body TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
    );")?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextItem {
    pub source_ref: String,
    pub source_type: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerSpec {
    pub task_id: String,
    pub role: String,
    pub objective: String,
    pub context: Vec<ContextItem>,
    pub profile: Profile,
    pub budget_microusd: u64,
    pub timeout_secs: u64,
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    #[serde(default)]
    pub allowed_credentials: Vec<String>,
    #[serde(default)]
    pub retry_deep: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub source_ref: String,
    pub source_type: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub filename: String,
    pub content: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryCandidate {
    pub content: String,
    pub category: String,
    pub confidence: f64,
    pub source_ref: String,
    pub sensitivity: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerResult {
    pub summary: String,
    pub findings: Vec<String>,
    pub confidence: f64,
    #[serde(default)]
    pub sources: Vec<Source>,
    #[serde(default)]
    pub artifacts: Vec<Artifact>,
    #[serde(default)]
    pub memory_candidates: Vec<MemoryCandidate>,
    #[serde(default)]
    pub recommended_skills: Vec<String>,
    #[serde(default)]
    pub errors: Vec<String>,
}

impl WorkerResult {
    pub fn validate(&self, spec: &WorkerSpec) -> Result<()> {
        text(&self.summary, 4000, "Worker summary")?;
        if !self.confidence.is_finite() || !(0.0..=1.0).contains(&self.confidence) {
            bail!("Invalid worker confidence");
        }
        if self.findings.len() > 100
            || self.sources.len() > 100
            || self.artifacts.len() > 16
            || self.memory_candidates.len() > 20
            || self.errors.len() > 30
            || self.recommended_skills.len() > 20
        {
            bail!("Worker result exceeds collection limits");
        }
        for finding in &self.findings {
            text(finding, 4000, "Finding")?;
        }
        for source in &self.sources {
            if !spec
                .context
                .iter()
                .any(|c| c.source_ref == source.source_ref && c.source_type == source.source_type)
            {
                bail!("Worker cited a source outside its supplied context");
            }
        }
        let mut names = std::collections::HashSet::new();
        for artifact in &self.artifacts {
            if artifact.filename.is_empty()
                || artifact.filename.len() > 100
                || artifact.filename.contains("..")
                || !artifact
                    .filename
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
                || artifact.content.len() > 1_000_000
                || !names.insert(&artifact.filename)
            {
                bail!("Unsafe, duplicate, or oversized worker artifact");
            }
        }
        for candidate in &self.memory_candidates {
            text(&candidate.content, 2000, "Memory candidate")?;
            text(&candidate.category, 100, "Memory category")?;
            if !candidate.confidence.is_finite()
                || !(0.0..=1.0).contains(&candidate.confidence)
                || !spec
                    .context
                    .iter()
                    .any(|c| c.source_ref == candidate.source_ref)
            {
                bail!("Invalid candidate confidence or provenance");
            }
            if !matches!(candidate.sensitivity.as_str(), "ordinary" | "sensitive") {
                bail!("Invalid candidate sensitivity");
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerRecord {
    pub id: String,
    pub owner_agent: AgentId,
    pub task_id: String,
    pub spec: WorkerSpec,
    pub status: String,
    pub result: Option<WorkerResult>,
    pub error: Option<String>,
    pub archive_path: Option<String>,
    pub active: bool,
}

pub(super) struct ActiveWorker {
    cancel: watch::Sender<bool>,
    done: watch::Receiver<bool>,
}

impl AgentHost {
    pub fn spawn_worker(
        &self,
        config: &Config,
        owner: AgentId,
        spec: WorkerSpec,
    ) -> Result<WorkerRecord> {
        let task = self.task(owner, &spec.task_id)?;
        if matches!(task.status, TaskStatus::Completed | TaskStatus::Cancelled)
            || !self.dependencies_complete(owner, &spec.task_id)?
        {
            bail!("Task cannot start a worker in its current state");
        }
        text(&spec.objective, 8000, "Worker objective")?;
        text(&spec.role, 100, "Worker role")?;
        if !spec.allowed_tools.is_empty() || !spec.allowed_credentials.is_empty() {
            bail!("P03 workers have no shell, network, database, or credential capabilities");
        }
        if spec.budget_microusd == 0
            || spec.timeout_secs == 0
            || spec.timeout_secs > 120
            || spec.context.len() > 32
            || spec.context.iter().map(|c| c.content.len()).sum::<usize>() > 64_000
        {
            bail!("Invalid worker bounds");
        }
        let mut refs = std::collections::HashSet::new();
        for item in &spec.context {
            text(&item.source_ref, 200, "Source reference")?;
            text(&item.content, 32_000, "Selected context")?;
            if !matches!(item.source_type.as_str(), "user" | "file" | "task")
                || !refs.insert(&item.source_ref)
            {
                bail!("Invalid or duplicate selected context source");
            }
        }
        let id = Uuid::new_v4().to_string();
        let dir = self.worker_workspace(owner, &id)?;
        let (cancel, mut cancelled) = watch::channel(false);
        let (done, receiver) = watch::channel(false);
        let expires = chrono::Utc::now().timestamp() + i64::try_from(spec.timeout_secs)? + 30;
        {
            let mut db = self.connection()?;
            let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            let (global_count,domain_count):(i64,i64)=tx.query_row("SELECT count(*),coalesce(sum(owner_agent=?1),0) FROM workers WHERE status IN ('queued','running') AND lease_expires_at>?2",params![owner.as_str(),chrono::Utc::now().timestamp()],|row|Ok((row.get(0)?,row.get(1)?)))?;
            if global_count >= 2 || domain_count >= 1 {
                drop(tx);
                let _ = std::fs::remove_dir(&dir);
                bail!("Worker concurrency limit reached: two globally, one per permanent agent");
            }
            tx.execute("INSERT INTO workers(id,owner_agent,task_id,spec,status,lease_expires_at) VALUES (?1,?2,?3,?4,'queued',?5)",params![id,owner.as_str(),spec.task_id,serde_json::to_string(&spec)?,expires])?;
            tx.execute("INSERT INTO worker_messages(worker_id,owner_agent,sender,recipient,message_type,body) VALUES (?1,?2,?2,?1,'delegate',?3)",params![id,owner.as_str(),serde_json::to_string(&spec)?])?;
            tx.execute(
                "INSERT INTO task_events(task_id,event,detail) VALUES (?1,'worker_spawned',?2)",
                params![spec.task_id, id],
            )?;
            tx.commit()?;
        }
        self.set_task_status(owner, &spec.task_id, TaskStatus::Investigating)?;
        self.active_workers
            .lock()
            .map_err(|_| anyhow::anyhow!("Worker registry poisoned"))?
            .insert(
                id.clone(),
                ActiveWorker {
                    cancel,
                    done: receiver,
                },
            );
        let host = self.clone();
        let config = config.clone();
        let worker_id = id.clone();
        tokio::spawn(async move {
            let operation = host.run_worker(&config, owner, &worker_id, &spec);
            let outcome = tokio::select! {
                result=tokio::time::timeout(std::time::Duration::from_secs(spec.timeout_secs),operation)=>match result {Ok(result)=>result,Err(_)=>Err(anyhow::anyhow!("Worker timed out"))},
                _=cancelled.changed()=>Err(anyhow::anyhow!("Worker cancelled")),
            };
            let status = if *cancelled.borrow() {
                "cancelled"
            } else if outcome.is_ok() {
                "completed"
            } else {
                "failed"
            };
            if let Err(error) = host.finish_worker(owner, &worker_id, status, outcome) {
                tracing::error!(worker_id, "Worker finalization failed: {error}");
                let _ = host.audit(
                    &worker_id,
                    owner,
                    "worker_finalize_failed",
                    "Archive or persistence failed; recover this worker",
                );
            }
            let _ = std::fs::remove_dir(&dir); // Only remove an empty ephemeral workspace; artifacts live in the archive.
            if let Ok(mut active) = host.active_workers.lock() {
                active.remove(&worker_id);
            }
            let _ = done.send(true);
        });
        self.worker(owner, &id)
    }

    fn worker_workspace(&self, owner: AgentId, id: &str) -> Result<std::path::PathBuf> {
        Uuid::parse_str(id)?;
        let dir = self.record(owner).workspace.join("workers");
        super::private_directory(&dir)?;
        let path = dir.join(id);
        super::private_directory(&path)?;
        Ok(path)
    }

    async fn run_worker(
        &self,
        config: &Config,
        owner: AgentId,
        id: &str,
        spec: &WorkerSpec,
    ) -> Result<WorkerResult> {
        self.connection()?.execute(
            "UPDATE workers SET status='running' WHERE id=?1 AND owner_agent=?2",
            params![id, owner.as_str()],
        )?;
        let system="You are a temporary task worker. Analyze ONLY the supplied context. You have no shell, network, filesystem, memory, or secret tools. Return exactly one JSON object with summary, findings (string array), confidence (0..1), sources [{source_ref,source_type}], artifacts [{filename,content}], memory_candidates [{content,category,confidence,source_ref,sensitivity: ordinary or sensitive}], recommended_skills (string array), errors (string array). Sources must reference supplied context. Distinguish facts from inference. Do not claim external actions. Empty arrays are valid.";
        let input = serde_json::to_string(spec)?;
        let history = vec![ChatMessage::system(system), ChatMessage::user(input)];
        let scope = InferenceScope {
            owner,
            task: Some(&spec.task_id),
            worker: Some(id),
            profile: spec.profile,
            budget_microusd: Some(spec.budget_microusd),
        };
        let first = self.infer(config, scope.clone(), &history).await;
        let raw = match first {
            Ok(raw) => raw,
            Err(_) if spec.retry_deep && spec.profile != Profile::Deep => {
                self.audit(
                    id,
                    owner,
                    "reasoning_escalated",
                    "Worker failure; explicit deep retry requested",
                )?;
                self.infer(
                    config,
                    InferenceScope {
                        profile: Profile::Deep,
                        ..scope
                    },
                    &history,
                )
                .await?
            }
            Err(error) => return Err(error),
        };
        self.connection()?.execute("INSERT INTO worker_messages(worker_id,owner_agent,sender,recipient,message_type,body) VALUES (?1,?2,?1,?2,'raw_result',?3)",params![id,owner.as_str(),raw])?;
        let result: WorkerResult =
            serde_json::from_str(&raw).context("Worker returned invalid completion JSON")?;
        result.validate(spec)?;
        Ok(result)
    }

    fn finish_worker(
        &self,
        owner: AgentId,
        id: &str,
        status: &str,
        outcome: Result<WorkerResult>,
    ) -> Result<()> {
        let (result, error) = match outcome {
            Ok(result) => (Some(result), None),
            Err(error) => (
                None,
                Some(crate::providers::sanitize_api_error(&error.to_string())),
            ),
        };
        let body = serde_json::to_string(
            &serde_json::json!({"status":status,"result":result,"error":error}),
        )?;
        {
            let mut db = self.connection()?;
            let tx = db.transaction()?;
            tx.execute("UPDATE workers SET status='archive_pending',result=?1,error=?2 WHERE id=?3 AND owner_agent=?4",params![result.as_ref().map(serde_json::to_string).transpose()?,error,id,owner.as_str()])?;
            tx.execute("INSERT INTO worker_messages(worker_id,owner_agent,sender,recipient,message_type,body) VALUES (?1,?2,?1,?2,'result',?3)",params![id,owner.as_str(),body])?;
            tx.execute("UPDATE model_usage SET status='interrupted_uncertain' WHERE worker_id=?1 AND status='reserved'",[id])?;
            tx.commit()?;
        }
        self.archive_worker(owner, id, status)?;
        Ok(())
    }

    pub fn worker(&self, owner: AgentId, id: &str) -> Result<WorkerRecord> {
        let db = self.connection()?;
        let row=db.query_row("SELECT task_id,spec,status,result,error,archive_path FROM workers WHERE id=?1 AND owner_agent=?2",params![id,owner.as_str()],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,Option<String>>(3)?,r.get::<_,Option<String>>(4)?,r.get::<_,Option<String>>(5)?))).optional()?.context("Worker not found in selected agent's domain")?;
        Ok(WorkerRecord {
            id: id.into(),
            owner_agent: owner,
            task_id: row.0,
            spec: serde_json::from_str(&row.1)?,
            active: matches!(row.2.as_str(), "queued" | "running"),
            status: row.2,
            result: row.3.map(|s| serde_json::from_str(&s)).transpose()?,
            error: row.4,
            archive_path: row.5,
        })
    }

    pub fn workers(&self, owner: AgentId) -> Result<Vec<WorkerRecord>> {
        let ids = {
            let db = self.connection()?;
            let mut q =
                db.prepare("SELECT id FROM workers WHERE owner_agent=?1 ORDER BY created_at,id")?;
            let collected = q
                .query_map([owner.as_str()], |r| r.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            collected
        };
        ids.iter().map(|id| self.worker(owner, id)).collect()
    }

    pub fn cancel_worker(&self, owner: AgentId, id: &str) -> Result<()> {
        self.worker(owner, id)?;
        let active = self
            .active_workers
            .lock()
            .map_err(|_| anyhow::anyhow!("Worker registry poisoned"))?;
        active
            .get(id)
            .context("Worker is not active in this host")?
            .cancel
            .send(true)?;
        self.audit(
            id,
            owner,
            "worker_cancel_requested",
            "Owner requested cancellation",
        )
    }

    pub async fn wait_worker(&self, owner: AgentId, id: &str) -> Result<WorkerRecord> {
        self.worker(owner, id)?;
        let receiver = {
            self.active_workers
                .lock()
                .map_err(|_| anyhow::anyhow!("Worker registry poisoned"))?
                .get(id)
                .map(|w| w.done.clone())
        };
        if let Some(mut receiver) = receiver {
            if !*receiver.borrow() {
                let _ = receiver.changed().await;
            }
        } else if self.worker(owner, id)?.active {
            bail!("Worker is active in another host; inspect status or recover after its lease expires");
        }
        self.worker(owner, id)
    }

    pub async fn wait_active_workers(&self, owner: AgentId) -> Result<()> {
        let local: std::collections::HashSet<_> = self
            .active_workers
            .lock()
            .map_err(|_| anyhow::anyhow!("Worker registry poisoned"))?
            .keys()
            .cloned()
            .collect();
        let ids = self
            .workers(owner)?
            .into_iter()
            .filter(|w| w.active && local.contains(&w.id))
            .map(|w| w.id)
            .collect::<Vec<_>>();
        for id in ids {
            self.wait_worker(owner, &id).await?;
        }
        Ok(())
    }

    pub fn retry_worker(&self, config: &Config, owner: AgentId, id: &str) -> Result<WorkerRecord> {
        let worker = self.worker(owner, id)?;
        if worker.active || worker.archive_path.is_none() {
            bail!("Only archived workers can be retried");
        }
        let mut spec = worker.spec;
        let task = self.task(owner, &spec.task_id)?;
        if matches!(task.status, TaskStatus::Completed | TaskStatus::Cancelled) {
            spec.task_id = self
                .create_task(
                    owner,
                    &format!("Retry: {}", task.title),
                    &task.description,
                    task.thread_id.as_deref(),
                )?
                .id;
        }
        self.audit(
            id,
            owner,
            "worker_retry_requested",
            "New worker receives original scoped inputs, with a fresh ID",
        )?;
        self.spawn_worker(config, owner, spec)
    }

    pub fn recover_workers(&self, owner: AgentId) -> Result<usize> {
        let ids = {
            let db = self.connection()?;
            let mut q=db.prepare("SELECT id FROM workers WHERE owner_agent=?1 AND (status='archive_pending' OR (status IN ('queued','running') AND lease_expires_at<?2))")?;
            let collected = q
                .query_map(
                    params![owner.as_str(), chrono::Utc::now().timestamp()],
                    |r| r.get::<_, String>(0),
                )?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            collected
        };
        let mut recovered = 0;
        for id in &ids {
            let active = self
                .active_workers
                .lock()
                .map_err(|_| anyhow::anyhow!("Worker registry poisoned"))?
                .contains_key(id);
            if active {
                continue;
            }
            let record = self.worker(owner, id)?;
            if record.status == "archive_pending" {
                let final_status: String = self.connection()?.query_row(
                    "SELECT json_extract(body,'$.status') FROM worker_messages WHERE worker_id=?1 AND message_type='result' ORDER BY sequence DESC LIMIT 1", [id], |r| r.get(0))?;
                self.archive_worker(owner, id, &final_status)?;
            } else {
                self.finish_worker(
                    owner,
                    id,
                    "interrupted",
                    Err(anyhow::anyhow!(
                        "Worker lease expired after runtime interruption"
                    )),
                )?;
            }
            let workspace = self.record(owner).workspace.join("workers").join(id);
            super::reject_link(&workspace)?;
            if workspace.exists() {
                std::fs::remove_dir(workspace)?;
            }
            recovered += 1;
        }
        Ok(recovered)
    }
}
