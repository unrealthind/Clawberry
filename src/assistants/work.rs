//! Owned tasks, durable Focus Threads, and scoped attachments.
use super::{AgentHost, AgentId, ChatMessage, MessageEnvelope};
use anyhow::{bail, Context, Result};
use rusqlite::{params, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub(super) fn migrate(tx: &Transaction<'_>) -> Result<()> {
    tx.execute_batch(
        "CREATE TABLE IF NOT EXISTS focus_threads (
            id TEXT PRIMARY KEY, owner_agent TEXT NOT NULL REFERENCES agents(id),
            title TEXT NOT NULL, summary TEXT NOT NULL DEFAULT '', revision INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS tasks (
            id TEXT PRIMARY KEY, owner_agent TEXT NOT NULL REFERENCES agents(id),
            title TEXT NOT NULL, description TEXT NOT NULL, status TEXT NOT NULL,
            thread_id TEXT REFERENCES focus_threads(id), priority TEXT NOT NULL,
            due_at TEXT, created_by TEXT NOT NULL, source TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS task_events (
            sequence INTEGER PRIMARY KEY AUTOINCREMENT, task_id TEXT NOT NULL REFERENCES tasks(id),
            event TEXT NOT NULL, detail TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS task_dependencies (
            task_id TEXT NOT NULL REFERENCES tasks(id), depends_on TEXT NOT NULL REFERENCES tasks(id),
            PRIMARY KEY(task_id, depends_on), CHECK(task_id <> depends_on)
        );
        CREATE TABLE IF NOT EXISTS thread_messages (
            sequence INTEGER PRIMARY KEY AUTOINCREMENT, thread_id TEXT NOT NULL REFERENCES focus_threads(id),
            role TEXT NOT NULL, envelope TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS attachments (
            id TEXT PRIMARY KEY, owner_agent TEXT NOT NULL REFERENCES agents(id),
            thread_id TEXT NOT NULL REFERENCES focus_threads(id), task_id TEXT REFERENCES tasks(id),
            filename TEXT NOT NULL, path TEXT NOT NULL, checksum TEXT NOT NULL, size INTEGER NOT NULL,
            source TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS thread_decisions (
            id TEXT PRIMARY KEY, owner_agent TEXT NOT NULL REFERENCES agents(id),
            thread_id TEXT NOT NULL REFERENCES focus_threads(id), content TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(owner_agent,thread_id,id)
        );
        CREATE INDEX IF NOT EXISTS decisions_thread ON thread_decisions(owner_agent,thread_id);
        CREATE INDEX IF NOT EXISTS task_owner ON tasks(owner_agent, status);
        CREATE INDEX IF NOT EXISTS thread_owner ON focus_threads(owner_agent);
        CREATE INDEX IF NOT EXISTS thread_history ON thread_messages(thread_id, sequence);"
    )?;
    Ok(())
}

pub(super) fn text(value: &str, max: usize, field: &str) -> Result<()> {
    if value.trim().is_empty() || value.len() > max {
        bail!("{field} must contain 1–{max} bytes of nonempty text");
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Queued,
    Investigating,
    Waiting,
    Blocked,
    NeedsManni,
    Approved,
    Executing,
    Completed,
    Cancelled,
}

impl TaskStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Investigating => "investigating",
            Self::Waiting => "waiting",
            Self::Blocked => "blocked",
            Self::NeedsManni => "needs_manni",
            Self::Approved => "approved",
            Self::Executing => "executing",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub owner_agent: AgentId,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub thread_id: Option<String>,
    pub priority: String,
    pub due_at: Option<String>,
    pub created_by: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusThread {
    pub id: String,
    pub owner_agent: AgentId,
    pub title: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub id: String,
    pub owner_agent: AgentId,
    pub thread_id: String,
    pub task_id: Option<String>,
    pub filename: String,
    pub checksum: String,
    pub size: usize,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadDecision {
    pub id: String,
    pub content: String,
    pub created_at: String,
}

impl AgentHost {
    /// Human-recorded decisions are immutable evidence, not execution approvals.
    /// Caller-generated IDs make an uncertain network retry idempotent.
    pub fn record_decision(
        &self,
        owner: AgentId,
        thread: &str,
        id: &str,
        content: &str,
    ) -> Result<Vec<ThreadDecision>> {
        self.thread(owner, thread)?;
        Uuid::parse_str(id)?;
        text(content, 4000, "Decision")?;
        let mut db = self.connection()?;
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let existing: Option<(String, String, String)> = tx
            .query_row(
                "SELECT owner_agent,thread_id,content FROM thread_decisions WHERE id=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        if let Some((agent, scope, saved)) = existing {
            if agent != owner.as_str() || scope != thread || saved != content {
                bail!("Decision identifier already used");
            }
        } else {
            tx.execute("INSERT INTO thread_decisions(id,owner_agent,thread_id,content) VALUES (?1,?2,?3,?4)", params![id,owner.as_str(),thread,content])?;
            tx.execute("INSERT INTO audit_events(request_id,owner_agent,event,detail) VALUES (?1,?2,'decision_recorded',?3)", params![id,owner.as_str(),thread])?;
        }
        tx.commit()?;
        drop(db);
        self.decisions(owner, thread)
    }

    pub fn decisions(&self, owner: AgentId, thread: &str) -> Result<Vec<ThreadDecision>> {
        self.thread(owner, thread)?;
        let db = self.connection()?;
        let mut query = db.prepare("SELECT id,content,created_at FROM thread_decisions WHERE owner_agent=?1 AND thread_id=?2 ORDER BY created_at,id")?;
        let rows = query.query_map(params![owner.as_str(), thread], |r| {
            Ok(ThreadDecision {
                id: r.get(0)?,
                content: r.get(1)?,
                created_at: r.get(2)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn set_task_metadata(
        &self,
        owner: AgentId,
        id: &str,
        priority: &str,
        due: Option<&str>,
    ) -> Result<()> {
        let task = self.task(owner, id)?;
        if matches!(task.status, TaskStatus::Completed | TaskStatus::Cancelled) {
            bail!("Terminal task metadata is immutable");
        }
        if !matches!(priority, "low" | "normal" | "high" | "urgent") {
            bail!("Invalid task priority");
        }
        if let Some(due) = due {
            chrono::DateTime::parse_from_rfc3339(due)
                .context("Due date must be RFC3339 with an explicit offset")?;
        }
        let mut db = self.connection()?;
        let tx = db.transaction()?;
        tx.execute("UPDATE tasks SET priority=?1,due_at=?2,updated_at=CURRENT_TIMESTAMP WHERE id=?3 AND owner_agent=?4",params![priority,due,id,owner.as_str()])?;
        tx.execute(
            "INSERT INTO task_events(task_id,event,detail) VALUES (?1,'metadata_changed',?2)",
            params![
                id,
                serde_json::to_string(&serde_json::json!({"priority":priority,"due_at":due}))?
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn task_details(&self, owner: AgentId, id: &str) -> Result<serde_json::Value> {
        let task = self.task(owner, id)?;
        let events = {
            let db = self.connection()?;
            let mut q=db.prepare("SELECT event,detail,created_at FROM task_events WHERE task_id=?1 ORDER BY sequence")?;
            let collected = q.query_map([id],|r|Ok(serde_json::json!({"event":r.get::<_,String>(0)?,"detail":r.get::<_,String>(1)?,"created_at":r.get::<_,String>(2)?})))?.collect::<rusqlite::Result<Vec<_>>>()?;
            collected
        };
        let workers = self
            .workers(owner)?
            .into_iter()
            .filter(|w| w.task_id == id)
            .collect::<Vec<_>>();
        let attachments = if let Some(thread) = &task.thread_id {
            self.attachments(owner, thread)?
                .into_iter()
                .filter(|a| a.task_id.as_deref() == Some(id))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        Ok(
            serde_json::json!({"task":task,"events":events,"workers":workers,"attachments":attachments}),
        )
    }
    pub fn create_thread(&self, owner: AgentId, title: &str) -> Result<FocusThread> {
        text(title, 200, "Thread title")?;
        let thread = FocusThread {
            id: Uuid::new_v4().to_string(),
            owner_agent: owner,
            title: title.into(),
            summary: String::new(),
        };
        self.connection()?.execute(
            "INSERT INTO focus_threads(id,owner_agent,title) VALUES (?1,?2,?3)",
            params![thread.id, owner.as_str(), title],
        )?;
        self.audit(&thread.id, owner, "thread_created", title)?;
        Ok(thread)
    }

    pub fn thread(&self, owner: AgentId, id: &str) -> Result<FocusThread> {
        self.connection()?
            .query_row(
                "SELECT title,summary FROM focus_threads WHERE id=?1 AND owner_agent=?2",
                params![id, owner.as_str()],
                |r| {
                    Ok(FocusThread {
                        id: id.into(),
                        owner_agent: owner,
                        title: r.get(0)?,
                        summary: r.get(1)?,
                    })
                },
            )
            .optional()?
            .context("Thread not found in selected agent's domain")
    }

    pub fn threads(&self, owner: AgentId) -> Result<Vec<FocusThread>> {
        let db = self.connection()?;
        let mut query=db.prepare("SELECT id,title,summary FROM focus_threads WHERE owner_agent=?1 ORDER BY created_at,id")?;
        let rows = query.query_map([owner.as_str()], |r| {
            Ok(FocusThread {
                id: r.get(0)?,
                owner_agent: owner,
                title: r.get(1)?,
                summary: r.get(2)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn summarize_thread(&self, owner: AgentId, id: &str, summary: &str) -> Result<()> {
        text(summary, 2000, "Thread summary")?;
        self.thread(owner, id)?;
        self.connection()?.execute("UPDATE focus_threads SET summary=?1,updated_at=CURRENT_TIMESTAMP WHERE id=?2 AND owner_agent=?3",params![summary,id,owner.as_str()])?;
        self.audit(
            id,
            owner,
            "thread_summary_updated",
            "Compact summary replaced",
        )
    }

    pub fn create_task(
        &self,
        owner: AgentId,
        title: &str,
        description: &str,
        thread: Option<&str>,
    ) -> Result<Task> {
        text(title, 200, "Task title")?;
        text(description, 16_000, "Task description")?;
        if let Some(id) = thread {
            self.thread(owner, id)?;
        }
        let task = Task {
            id: Uuid::new_v4().to_string(),
            owner_agent: owner,
            title: title.into(),
            description: description.into(),
            status: TaskStatus::Queued,
            thread_id: thread.map(str::to_owned),
            priority: "normal".into(),
            due_at: None,
            created_by: owner.as_str().into(),
            source: "internal".into(),
        };
        let mut db = self.connection()?;
        let tx = db.transaction()?;
        tx.execute("INSERT INTO tasks(id,owner_agent,title,description,status,thread_id,priority,created_by,source) VALUES (?1,?2,?3,?4,'queued',?5,'normal',?2,'internal')",params![task.id,owner.as_str(),title,description,thread])?;
        tx.execute("INSERT INTO task_events(task_id,event,detail) VALUES (?1,'created','Internal task created')",[&task.id])?;
        tx.commit()?;
        Ok(task)
    }

    /// Edit owned task fields atomically and retain the reviewed payload in its audit history.
    #[allow(clippy::too_many_arguments)]
    pub fn edit_task(
        &self,
        owner: AgentId,
        id: &str,
        title: &str,
        description: &str,
        thread: Option<&str>,
        priority: &str,
        due: Option<&str>,
    ) -> Result<()> {
        self.task(owner, id)?;
        text(title, 200, "Task title")?;
        text(description, 16_000, "Task description")?;
        if let Some(thread) = thread {
            self.thread(owner, thread)?;
        }
        if !matches!(priority, "low" | "normal" | "high" | "urgent") {
            bail!("Invalid task priority");
        }
        if let Some(due) = due {
            chrono::DateTime::parse_from_rfc3339(due)
                .context("Due date must be RFC3339 with an explicit offset")?;
        }
        let mut db = self.connection()?;
        let tx = db.transaction()?;
        let changed = tx.execute("UPDATE tasks SET title=?1,description=?2,thread_id=?3,priority=?4,due_at=?5,updated_at=CURRENT_TIMESTAMP WHERE id=?6 AND owner_agent=?7 AND status NOT IN ('completed','cancelled')", params![title,description,thread,priority,due,id,owner.as_str()])?;
        if changed != 1 {
            bail!("Terminal task is immutable; create a retry task");
        }
        tx.execute("INSERT INTO task_events(task_id,event,detail) VALUES (?1,'edited',?2)", params![id,serde_json::to_string(&serde_json::json!({"title":title,"description":description,"thread_id":thread,"priority":priority,"due_at":due}))?])?;
        tx.commit()?;
        Ok(())
    }

    pub fn task(&self, owner: AgentId, id: &str) -> Result<Task> {
        let db = self.connection()?;
        let row=db.query_row("SELECT title,description,status,thread_id,priority,due_at,created_by,source FROM tasks WHERE id=?1 AND owner_agent=?2",params![id,owner.as_str()], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,Option<String>>(3)?,r.get::<_,String>(4)?,r.get::<_,Option<String>>(5)?,r.get::<_,String>(6)?,r.get::<_,String>(7)?))).optional()?.context("Task not found in selected agent's domain")?;
        Ok(Task {
            id: id.into(),
            owner_agent: owner,
            title: row.0,
            description: row.1,
            status: serde_json::from_value(serde_json::Value::String(row.2))?,
            thread_id: row.3,
            priority: row.4,
            due_at: row.5,
            created_by: row.6,
            source: row.7,
        })
    }

    pub fn tasks(&self, owner: AgentId) -> Result<Vec<Task>> {
        let ids = {
            let db = self.connection()?;
            let mut q =
                db.prepare("SELECT id FROM tasks WHERE owner_agent=?1 ORDER BY created_at,id")?;
            let collected = q
                .query_map([owner.as_str()], |r| r.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            collected
        };
        ids.iter().map(|id| self.task(owner, id)).collect()
    }

    pub fn set_task_status(&self, owner: AgentId, id: &str, status: TaskStatus) -> Result<()> {
        let task = self.task(owner, id)?;
        if matches!(task.status, TaskStatus::Completed | TaskStatus::Cancelled) {
            bail!("Terminal task is immutable; create a retry task");
        }
        if matches!(status, TaskStatus::Approved | TaskStatus::Executing) {
            bail!("Execution/approval states require the later approval service");
        }
        if status == TaskStatus::Completed && !self.dependencies_complete(owner, id)? {
            bail!("Task has incomplete dependencies");
        }
        let mut db = self.connection()?;
        let tx = db.transaction()?;
        if status == TaskStatus::Completed && tx.query_row("SELECT EXISTS(SELECT 1 FROM task_dependencies d JOIN tasks t ON t.id=d.depends_on WHERE d.task_id=?1 AND t.status<>'completed')",[id],|r|r.get::<_,bool>(0))? {
            bail!("Task acquired an incomplete dependency while completing");
        }
        let changed = tx.execute("UPDATE tasks SET status=?1,updated_at=CURRENT_TIMESTAMP WHERE id=?2 AND owner_agent=?3 AND status NOT IN ('completed','cancelled')",params![status.as_str(),id,owner.as_str()])?;
        if changed != 1 {
            bail!("Task became terminal while updating");
        }
        tx.execute(
            "INSERT INTO task_events(task_id,event,detail) VALUES (?1,'status_changed',?2)",
            params![id, status.as_str()],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn add_dependency(&self, owner: AgentId, task: &str, dependency: &str) -> Result<()> {
        self.task(owner, task)?;
        self.task(owner, dependency)?;
        let mut db = self.connection()?;
        let tx = db.transaction()?;
        let terminal: bool = tx.query_row(
            "SELECT status IN ('completed','cancelled') FROM tasks WHERE id=?1 AND owner_agent=?2",
            params![task, owner.as_str()],
            |r| r.get(0),
        )?;
        if terminal {
            bail!("Terminal task dependencies are immutable");
        }
        let cycle:bool=tx.query_row("WITH RECURSIVE chain(id) AS (SELECT ?1 UNION SELECT depends_on FROM task_dependencies JOIN chain ON task_id=chain.id) SELECT EXISTS(SELECT 1 FROM chain WHERE id=?2)",params![dependency,task],|r|r.get(0))?;
        if cycle {
            bail!("Task dependency would create a cycle");
        }
        tx.execute(
            "INSERT INTO task_dependencies(task_id,depends_on) VALUES (?1,?2)",
            params![task, dependency],
        )?;
        tx.execute(
            "INSERT INTO task_events(task_id,event,detail) VALUES (?1,'dependency_added',?2)",
            params![task, dependency],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn dependencies_complete(&self, owner: AgentId, task: &str) -> Result<bool> {
        self.task(owner, task)?;
        Ok(!self.connection()?.query_row("SELECT EXISTS(SELECT 1 FROM task_dependencies d JOIN tasks t ON t.id=d.depends_on WHERE d.task_id=?1 AND t.status<>'completed')",[task],|r|r.get::<_,bool>(0))?)
    }

    pub fn relevant_threads(&self, owner: AgentId, message: &str) -> Result<Vec<FocusThread>> {
        let words: Vec<_> = message
            .split_whitespace()
            .filter(|w| w.len() >= 3)
            .map(str::to_lowercase)
            .collect();
        Ok(self
            .threads(owner)?
            .into_iter()
            .filter(|t| {
                words
                    .iter()
                    .any(|w| t.title.to_lowercase().contains(w) || t.id == *w)
            })
            .take(3)
            .collect())
    }

    pub fn thread_history(&self, owner: AgentId, id: &str) -> Result<Vec<ChatMessage>> {
        self.thread(owner, id)?;
        let db = self.connection()?;
        let mut q=db.prepare("SELECT role,envelope FROM (SELECT sequence,role,envelope FROM thread_messages WHERE thread_id=?1 ORDER BY sequence DESC LIMIT 50) ORDER BY sequence")?;
        let mut result = Vec::new();
        for row in q.query_map([id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })? {
            let (role, json) = row?;
            let message: MessageEnvelope = serde_json::from_str(&json)?;
            if message.owner_agent != owner || message.session_id != id {
                bail!("Thread message ownership mismatch");
            }
            result.push(ChatMessage {
                role,
                content: message.content,
            });
        }
        Ok(result)
    }

    pub fn put_attachment(
        &self,
        owner: AgentId,
        thread: &str,
        task: Option<&str>,
        filename: &str,
        data: &[u8],
    ) -> Result<Attachment> {
        self.thread(owner, thread)?;
        if let Some(id) = task {
            if self.task(owner, id)?.thread_id.as_deref() != Some(thread) {
                bail!("Attachment task/thread mismatch");
            }
        }
        text(filename, 200, "Filename")?;
        if filename.contains(['/', '\\'])
            || filename.chars().any(char::is_control)
            || data.is_empty()
            || data.len() > 5_000_000
        {
            bail!("Unsafe filename or attachment exceeds 5 MB");
        }
        let attachment = Attachment {
            id: Uuid::new_v4().to_string(),
            owner_agent: owner,
            thread_id: thread.into(),
            task_id: task.map(str::to_owned),
            filename: filename.into(),
            checksum: hex::encode(Sha256::digest(data)),
            size: data.len(),
            source: "user_file".into(),
        };
        let dir = self.record(owner).workspace.join("files");
        super::private_directory(&dir)?;
        let path = dir.join(&attachment.id);
        std::fs::write(&path, data)?;
        self.connection()?.execute("INSERT INTO attachments(id,owner_agent,thread_id,task_id,filename,path,checksum,size,source) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",params![attachment.id,owner.as_str(),thread,task,filename,path.to_string_lossy(),attachment.checksum,i64::try_from(data.len())?,attachment.source])?;
        self.audit(&attachment.id, owner, "attachment_created", thread)?;
        Ok(attachment)
    }

    pub fn attachment_bytes(&self, owner: AgentId, id: &str) -> Result<Vec<u8>> {
        Uuid::parse_str(id)?;
        let (checksum, size): (String, usize) = self.connection()?.query_row(
            "SELECT checksum,size FROM attachments WHERE id=?1 AND owner_agent=?2",
            params![id, owner.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let directory = self.record(owner).workspace.join("files");
        super::reject_link(&directory)?;
        let root = directory.canonicalize()?;
        let path = root.join(id);
        super::reject_link(&path)?;
        if !path.canonicalize()?.starts_with(&root) || path.metadata()?.len() > 5_000_000 {
            bail!("Attachment escaped scope or exceeds limit");
        }
        let bytes = std::fs::read(path)?;
        if bytes.len() != size || hex::encode(Sha256::digest(&bytes)) != checksum {
            bail!("Attachment integrity check failed");
        }
        self.audit(
            id,
            owner,
            "attachment_read",
            "Authenticated owner-scoped download",
        )?;
        Ok(bytes)
    }

    pub fn attachments(&self, owner: AgentId, thread: &str) -> Result<Vec<Attachment>> {
        self.thread(owner, thread)?;
        let db = self.connection()?;
        let mut q=db.prepare("SELECT id,task_id,filename,checksum,size,source FROM attachments WHERE owner_agent=?1 AND thread_id=?2")?;
        let rows = q.query_map(params![owner.as_str(), thread], |r| {
            Ok(Attachment {
                id: r.get(0)?,
                owner_agent: owner,
                thread_id: thread.into(),
                task_id: r.get(1)?,
                filename: r.get(2)?,
                checksum: r.get(3)?,
                size: r.get(4)?,
                source: r.get(5)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }
}
