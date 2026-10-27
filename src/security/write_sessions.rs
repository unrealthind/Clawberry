//! Durable local write authorization and outcome ledger.
//! No TOTP codes or gateway bearer tokens are accepted or retained here.
//! The trusted gateway adapter (not an agent operation) must attest authorization.
use crate::assistants::AgentId;
use anyhow::{bail, Result};
use rusqlite::{params, Connection, TransactionBehavior};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

const SCHEMA_VERSION: i64 = 1;

const IMAP_OPERATIONS: &[&str] = &[
    "folder.create",
    "folder.rename",
    "email.move",
    "email.copy",
    "email.archive",
    "email.mark_read",
    "email.mark_unread",
    "email.flag",
    "email.unflag",
    "email.tag_add",
    "email.tag_remove",
];
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum WriteManifest {
    SingleAction {
        operation: String,
        payload: serde_json::Value,
    },
    BoundedSession {
        budgets: BTreeMap<String, u32>,
        mailboxes: BTreeSet<String>,
        message_uids: BTreeMap<String, BTreeSet<u32>>,
        archive_mailbox: String,
    },
}
impl WriteManifest {
    pub(crate) fn supports_operation(operation: &str) -> bool {
        IMAP_OPERATIONS.contains(&operation)
            || ["email.send", "email.reply", "email.forward"].contains(&operation)
    }
    pub(crate) fn operations(&self) -> Vec<&str> {
        match self {
            Self::SingleAction { operation, .. } => vec![operation.as_str()],
            Self::BoundedSession { budgets, .. } => budgets.keys().map(String::as_str).collect(),
        }
    }
    pub fn hash(&self) -> Result<String> {
        Ok(hex::encode(Sha256::digest(serde_json::to_vec(
            &serde_json::to_value(self)?,
        )?)))
    }
    pub(crate) fn validate(&self) -> Result<()> {
        match self {
            Self::SingleAction { operation, payload } => {
                if !["email.send", "email.reply", "email.forward"].contains(&operation.as_str())
                    || !payload.is_object()
                    || serde_json::to_vec(payload)?.len() > 32_000
                {
                    bail!("Invalid single-email manifest");
                }
                let object = payload.as_object().unwrap();
                if object.keys().any(|key| {
                    ![
                        "to",
                        "cc",
                        "bcc",
                        "subject",
                        "body",
                        "text",
                        "html",
                        "message_id",
                    ]
                    .contains(&key.as_str())
                }) {
                    bail!("Unreviewed draft fields, connection settings, or attachments are not supported");
                }
                for (key, value) in object {
                    if ["to", "cc", "bcc"].contains(&key.as_str()) {
                        let addresses = value.as_array().ok_or_else(|| {
                            anyhow::anyhow!("Recipients must be an explicit list")
                        })?;
                        if addresses.is_empty()
                            || addresses.len() > 20
                            || addresses.iter().any(|v| {
                                v.as_str().is_none_or(|s| {
                                    s.len() > 254
                                        || s.is_empty()
                                        || s.matches('@').count() != 1
                                        || s.chars().any(|c| c.is_control() || c.is_whitespace())
                                        || s.contains([';', ','])
                                })
                            })
                        {
                            bail!("Invalid reviewed recipient list")
                        }
                    } else {
                        let text = value
                            .as_str()
                            .ok_or_else(|| anyhow::anyhow!("Draft fields must be strings"))?;
                        let header = ["subject", "message_id"].contains(&key.as_str());
                        if text.len() > if header { 1000 } else { 20_000 }
                            || (header && text.chars().any(char::is_control))
                        {
                            bail!("Draft field exceeds bounds or contains header injection")
                        }
                    }
                }
                if !object.contains_key("to") {
                    bail!("Email writes require explicit reviewed recipients")
                }
                if operation != "email.send" && !object.contains_key("message_id") {
                    bail!("Reply/forward requires a reviewed source message identity")
                }
                if !["body", "text", "html"]
                    .iter()
                    .any(|key| object.contains_key(*key))
                {
                    bail!("Exact draft content is required")
                }
            }
            Self::BoundedSession {
                budgets,
                mailboxes,
                message_uids,
                archive_mailbox,
            } => {
                if budgets.is_empty()
                    || budgets.len() > IMAP_OPERATIONS.len()
                    || mailboxes.is_empty()
                    || mailboxes.len() > 100
                    || !mailboxes.contains(archive_mailbox)
                    || budgets.iter().any(|(op, count)| {
                        !IMAP_OPERATIONS.contains(&op.as_str()) || *count == 0 || *count > 100
                    })
                    || mailboxes.iter().any(|name| !valid_mailbox(name))
                    || message_uids.iter().any(|(name, uids)| {
                        !mailboxes.contains(name)
                            || uids.is_empty()
                            || uids.len() > 1000
                            || uids.iter().any(|uid| *uid == 0 || *uid > 2_147_483_647)
                    })
                {
                    bail!("Invalid bounded mailbox resource manifest");
                }
            }
        }
        Ok(())
    }
    fn admits(&self, operation: &str, payload: &serde_json::Value) -> Result<()> {
        match self {
            Self::SingleAction {
                operation: approved,
                payload: reviewed,
            } => {
                if operation != approved || payload != reviewed {
                    bail!("Single-action payload changed after review");
                }
            }
            Self::BoundedSession {
                budgets,
                mailboxes,
                message_uids,
                archive_mailbox,
            } => {
                if !budgets.contains_key(operation) {
                    bail!("Operation is outside the reviewed manifest");
                }
                let object = payload
                    .as_object()
                    .ok_or_else(|| anyhow::anyhow!("Action payload must be an object"))?;
                let keys: &[&str] = match operation {
                    "folder.create" => &["path"],
                    "folder.rename" => &["from", "to"],
                    "email.move" | "email.copy" => {
                        &["source_mailbox", "destination_mailbox", "uid"]
                    }
                    "email.archive" => &["source_mailbox", "uid"],
                    "email.tag_add" | "email.tag_remove" => &["mailbox", "uid", "tag"],
                    _ => &["mailbox", "uid"],
                };
                if object.len() != keys.len() || keys.iter().any(|key| !object.contains_key(*key)) {
                    bail!("Action payload schema mismatch");
                }
                for key in keys.iter().filter(|key| !["uid", "tag"].contains(key)) {
                    let name = object[*key]
                        .as_str()
                        .ok_or_else(|| anyhow::anyhow!("Mailbox must be a string"))?;
                    if !valid_mailbox(name) || !mailboxes.contains(name) {
                        bail!("Mailbox is outside the reviewed resource scope");
                    }
                }
                if operation.starts_with("email.") {
                    let source = object
                        .get("source_mailbox")
                        .or_else(|| object.get("mailbox"))
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("");
                    let uid = object
                        .get("uid")
                        .and_then(serde_json::Value::as_u64)
                        .and_then(|n| u32::try_from(n).ok())
                        .ok_or_else(|| anyhow::anyhow!("UID must be a reviewed integer"))?;
                    if !message_uids
                        .get(source)
                        .is_some_and(|uids| uids.contains(&uid))
                    {
                        bail!("Message UID is outside the reviewed resource scope");
                    }
                    if operation == "email.archive" && !mailboxes.contains(archive_mailbox) {
                        bail!("Archive mailbox is not reviewed");
                    }
                    if let Some(tag) = object.get("tag") {
                        let tag = tag.as_str().unwrap_or("");
                        if tag.is_empty()
                            || tag.len() > 64
                            || !tag
                                .bytes()
                                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
                            || !tag.as_bytes()[0].is_ascii_alphanumeric()
                        {
                            bail!("Invalid mailbox tag");
                        }
                    }
                }
            }
        }
        Ok(())
    }
    fn budgets(&self) -> BTreeMap<String, u32> {
        match self {
            Self::SingleAction { operation, .. } => BTreeMap::from([(operation.clone(), 1)]),
            Self::BoundedSession { budgets, .. } => budgets.clone(),
        }
    }
}
fn valid_mailbox(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 255
        && name.trim() == name
        && !name.contains(['*', '%'])
        && !name.chars().any(char::is_control)
        && name.split('/').count() <= 8
        && name
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != ".." && part.len() <= 100)
}

/// Constructed exclusively by trusted authorization code after gateway success.
/// Public model/API payloads cannot deserialize or construct this attestation.
pub(crate) struct VerifiedWriteGrant {
    pub id: String,
    pub agent: AgentId,
    pub task_id: String,
    pub broker_identity: String,
    pub manifest: WriteManifest,
    pub reviewed_hash: String,
    pub expires_at: i64,
    pub gateway_request_id: String,
    pub gateway_session_id: String,
    pub gateway_audit_id: String,
    pub credential_reference: String,
}
pub struct WriteSessionStore {
    db: Mutex<Connection>,
}
#[derive(Debug, Serialize)]
pub struct SessionStatus {
    pub id: String,
    pub mode: String,
    pub expires_at: i64,
    pub revoked: bool,
    pub gateway_end_confirmed: Option<bool>,
    pub remaining: BTreeMap<String, u32>,
    pub actions: Vec<WriteActionStatus>,
}
#[derive(Debug, Serialize)]
pub struct WriteActionStatus {
    pub id: String,
    pub operation: String,
    pub status: String,
    pub gateway_action_id: Option<String>,
    pub gateway_audit_id: Option<String>,
    pub n8n_execution_id: Option<String>,
}
#[derive(Clone, Copy)]
pub enum WriteOutcome {
    Succeeded,
    Failed,
    Uncertain,
}
impl WriteSessionStore {
    pub(crate) fn uncertain_action_hash(
        &self,
        id: &str,
        session: &str,
        owner: AgentId,
        task: &str,
    ) -> Result<String> {
        let db = self
            .db
            .lock()
            .map_err(|_| anyhow::anyhow!("Write database lock poisoned"))?;
        Ok(db.query_row("SELECT payload_hash FROM write_actions WHERE id=?1 AND session_id=?2 AND agent=?3 AND task_id=?4 AND status='uncertain'",params![id,session,owner.as_str(),task],|r|r.get(0))?)
    }
    pub(crate) fn gateway_scope(
        &self,
        id: &str,
        agent: AgentId,
        task: &str,
    ) -> Result<(String, String)> {
        let db = self
            .db
            .lock()
            .map_err(|_| anyhow::anyhow!("Write database lock poisoned"))?;
        Ok(db.query_row("SELECT gateway_session_id,credential_reference FROM write_sessions WHERE id=?1 AND agent=?2 AND task_id=?3",params![id,agent.as_str(),task],|r|Ok((r.get(0)?,r.get(1)?)))?)
    }
    pub fn open(path: &std::path::Path) -> Result<Self> {
        if path
            .symlink_metadata()
            .is_ok_and(|meta| meta.file_type().is_symlink())
        {
            bail!("Write authorization storage cannot be a symlink");
        }
        let mut db = Connection::open(path)?;
        db.busy_timeout(std::time::Duration::from_secs(5))?;
        let version: i64 = db.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if !(0..=SCHEMA_VERSION).contains(&version) {
            bail!("Unsupported write authorization schema; restore a matching backup");
        }
        db.execute_batch("PRAGMA foreign_keys=ON;
            CREATE TABLE IF NOT EXISTS write_sessions(id TEXT PRIMARY KEY,agent TEXT NOT NULL,task_id TEXT NOT NULL,broker_identity TEXT NOT NULL,manifest TEXT NOT NULL,manifest_hash TEXT NOT NULL,expires_at INTEGER NOT NULL,revoked INTEGER NOT NULL DEFAULT 0,gateway_request_id TEXT NOT NULL UNIQUE,gateway_session_id TEXT NOT NULL UNIQUE,gateway_audit_id TEXT NOT NULL,credential_reference TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS write_budgets(session_id TEXT NOT NULL REFERENCES write_sessions(id),operation TEXT NOT NULL,remaining INTEGER NOT NULL CHECK(remaining>=0),PRIMARY KEY(session_id,operation));
            CREATE TABLE IF NOT EXISTS write_actions(id TEXT PRIMARY KEY,session_id TEXT NOT NULL REFERENCES write_sessions(id),agent TEXT NOT NULL,task_id TEXT NOT NULL,operation TEXT NOT NULL,payload_hash TEXT NOT NULL,status TEXT NOT NULL,gateway_action_id TEXT,n8n_execution_id TEXT,lease_expires_at INTEGER NOT NULL);
            CREATE TABLE IF NOT EXISTS write_audit(sequence INTEGER PRIMARY KEY AUTOINCREMENT,session_id TEXT NOT NULL,action_id TEXT,event TEXT NOT NULL,created_at INTEGER NOT NULL);")?;
        // Expired dispatch leases have uncertain outcomes. A second process
        // opening this database must not invalidate another live dispatch.
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let version: i64 = tx.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if !(0..=SCHEMA_VERSION).contains(&version) {
            bail!("Unsupported write authorization schema; restore a matching backup");
        }
        let has_audit_column = {
            let mut statement = tx.prepare("PRAGMA table_info(write_actions)")?;
            let columns = statement
                .query_map([], |row| row.get::<_, String>(1))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            columns.iter().any(|name| name == "gateway_audit_id")
        };
        if !has_audit_column {
            if version == SCHEMA_VERSION {
                bail!("Write authorization schema is incomplete");
            }
            tx.execute_batch(
                "ALTER TABLE write_actions ADD COLUMN gateway_audit_id TEXT;
                UPDATE write_actions SET gateway_audit_id=gateway_action_id,
                gateway_action_id=CASE WHEN status='succeeded' THEN id ELSE NULL END;",
            )?;
        }
        tx.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        tx.execute("INSERT INTO write_audit(session_id,action_id,event,created_at) SELECT session_id,id,'restart_outcome_uncertain',?1 FROM write_actions WHERE status='reserved' AND lease_expires_at<=?1",[chrono::Utc::now().timestamp()])?;
        tx.execute(
            "UPDATE write_actions SET status='uncertain' WHERE status='reserved' AND lease_expires_at<=?1",
            [chrono::Utc::now().timestamp()],
        )?;
        tx.commit()?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        }
        Ok(Self { db: Mutex::new(db) })
    }
    pub(crate) fn record_verified(&self, grant: &VerifiedWriteGrant) -> Result<()> {
        grant.manifest.validate()?;
        let now = chrono::Utc::now().timestamp();
        if grant.expires_at <= now
            || grant.expires_at > now + 300
            || grant.manifest.hash()? != grant.reviewed_hash
            || [
                &grant.id,
                &grant.task_id,
                &grant.broker_identity,
                &grant.gateway_request_id,
                &grant.gateway_session_id,
                &grant.gateway_audit_id,
                &grant.credential_reference,
            ]
            .iter()
            .any(|value| value.trim().is_empty())
        {
            bail!("Invalid verified gateway grant");
        }
        let mut db = self
            .db
            .lock()
            .map_err(|_| anyhow::anyhow!("Write authorization database poisoned"))?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute("INSERT INTO write_sessions(id,agent,task_id,broker_identity,manifest,manifest_hash,expires_at,gateway_request_id,gateway_session_id,gateway_audit_id,credential_reference) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",params![grant.id,grant.agent.as_str(),grant.task_id,grant.broker_identity,serde_json::to_string(&grant.manifest)?,grant.reviewed_hash,grant.expires_at,grant.gateway_request_id,grant.gateway_session_id,grant.gateway_audit_id,grant.credential_reference])?;
        for (op, count) in grant.manifest.budgets() {
            tx.execute(
                "INSERT INTO write_budgets(session_id,operation,remaining) VALUES(?1,?2,?3)",
                params![grant.id, op, count],
            )?;
        }
        tx.execute("INSERT INTO write_audit(session_id,event,created_at) VALUES(?1,'gateway_authorized',?2)",params![grant.id,now])?;
        tx.commit()?;
        Ok(())
    }
    /// Call only AFTER deterministic policy and Guard. Does not dispatch a write.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn reserve(
        &self,
        session: &str,
        action_id: &str,
        agent: AgentId,
        task: &str,
        broker: &str,
        operation: &str,
        payload: &serde_json::Value,
    ) -> Result<()> {
        let mut db = self
            .db
            .lock()
            .map_err(|_| anyhow::anyhow!("Write authorization database poisoned"))?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let manifest:String=tx.query_row("SELECT manifest FROM write_sessions WHERE id=?1 AND agent=?2 AND task_id=?3 AND broker_identity=?4 AND expires_at>?5 AND revoked=0",params![session,agent.as_str(),task,broker,chrono::Utc::now().timestamp()],|r|r.get(0))?;
        let manifest: WriteManifest = serde_json::from_str(&manifest)?;
        manifest.admits(operation, payload)?;
        let payload_hash = hex::encode(Sha256::digest(serde_json::to_vec(payload)?));
        let unresolved:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM write_actions WHERE agent=?1 AND operation=?2 AND payload_hash=?3 AND status IN ('reserved','uncertain'))",params![agent.as_str(),operation,payload_hash],|r|r.get(0))?;
        if unresolved {
            bail!("Prior equivalent write has an unresolved outcome; reconcile before replay");
        }
        if tx.execute("UPDATE write_budgets SET remaining=remaining-1 WHERE session_id=?1 AND operation=?2 AND remaining>0",params![session,operation])?!=1 { bail!("Reviewed operation budget exhausted"); }
        tx.execute("INSERT INTO write_actions(id,session_id,agent,task_id,operation,payload_hash,status,lease_expires_at) VALUES(?1,?2,?3,?4,?5,?6,'reserved',?7)",params![action_id,session,agent.as_str(),task,operation,payload_hash,chrono::Utc::now().timestamp()+60])?;
        tx.execute("INSERT INTO write_audit(session_id,action_id,event,created_at) VALUES(?1,?2,'write_reserved',?3)",params![session,action_id,chrono::Utc::now().timestamp()])?;
        tx.commit()?;
        Ok(())
    }
    pub(crate) fn record_outcome(
        &self,
        action: &str,
        outcome: WriteOutcome,
        gateway_action_id: Option<&str>,
        gateway_audit_id: Option<&str>,
        execution_id: Option<&str>,
    ) -> Result<()> {
        if matches!(outcome, WriteOutcome::Succeeded)
            && (gateway_action_id != Some(action)
                || !gateway_audit_id.is_some_and(crate::n8n::valid_correlation_id)
                || !execution_id.is_some_and(crate::n8n::valid_correlation_id))
        {
            bail!("Confirmed write requires exact action, audit and execution correlation");
        }
        let state = match outcome {
            WriteOutcome::Succeeded => "succeeded",
            WriteOutcome::Failed => "failed",
            WriteOutcome::Uncertain => "uncertain",
        };
        let mut db = self
            .db
            .lock()
            .map_err(|_| anyhow::anyhow!("Write authorization database poisoned"))?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if tx.execute("UPDATE write_actions SET status=?1,gateway_action_id=?2,n8n_execution_id=?3,gateway_audit_id=?4 WHERE id=?5 AND status='reserved'",params![state,gateway_action_id,execution_id,gateway_audit_id,action])?!=1{bail!("Write outcome is not in a dispatchable state");}
        tx.execute("INSERT INTO write_audit(session_id,action_id,event,created_at) SELECT session_id,id,?1,?2 FROM write_actions WHERE id=?3",params![state,chrono::Utc::now().timestamp(),action])?;
        tx.commit()?;
        Ok(())
    }
    pub(crate) fn reconcile(
        &self,
        action: &str,
        succeeded: bool,
        evidence_ref: &str,
    ) -> Result<()> {
        if evidence_ref.trim().is_empty() {
            bail!("Reconciliation requires durable read-side evidence");
        }
        let mut db = self
            .db
            .lock()
            .map_err(|_| anyhow::anyhow!("Write authorization database poisoned"))?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if tx.execute(
            "UPDATE write_actions SET status=?1 WHERE id=?2 AND status='uncertain'",
            params![
                if succeeded {
                    "succeeded"
                } else {
                    "reconciled_not_applied"
                },
                action
            ],
        )? != 1
        {
            bail!("Action does not need reconciliation");
        }
        tx.execute("INSERT INTO write_audit(session_id,action_id,event,created_at) SELECT session_id,id,'write_reconciled',?1 FROM write_actions WHERE id=?2",params![chrono::Utc::now().timestamp(),action])?;
        // Evidence must remain reconstructable without recording raw mailbox data.
        tx.execute_batch("CREATE TABLE IF NOT EXISTS write_reconciliation(action_id TEXT PRIMARY KEY REFERENCES write_actions(id),evidence_ref TEXT NOT NULL);")?;
        tx.execute(
            "INSERT INTO write_reconciliation(action_id,evidence_ref) VALUES(?1,?2)",
            params![action, evidence_ref],
        )?;
        tx.commit()?;
        Ok(())
    }
    pub fn end(&self, id: &str, agent: AgentId, task: &str) -> Result<()> {
        let mut db = self
            .db
            .lock()
            .map_err(|_| anyhow::anyhow!("Write authorization database poisoned"))?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if tx.execute(
            "UPDATE write_sessions SET revoked=1 WHERE id=?1 AND agent=?2 AND task_id=?3",
            params![id, agent.as_str(), task],
        )? != 1
        {
            bail!("Session not found in this owner/task scope");
        }
        tx.execute(
            "INSERT INTO write_audit(session_id,event,created_at) VALUES(?1,'session_ended',?2)",
            params![id, chrono::Utc::now().timestamp()],
        )?;
        tx.commit()?;
        Ok(())
    }
    pub fn status(&self, id: &str, agent: AgentId, task: &str) -> Result<SessionStatus> {
        let db = self
            .db
            .lock()
            .map_err(|_| anyhow::anyhow!("Write authorization database poisoned"))?;
        let (manifest,expires_at,revoked):(String,i64,bool)=db.query_row("SELECT manifest,expires_at,revoked FROM write_sessions WHERE id=?1 AND agent=?2 AND task_id=?3",params![id,agent.as_str(),task],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;
        let manifest: WriteManifest = serde_json::from_str(&manifest)?;
        let closure: Option<String> = db.query_row("SELECT (SELECT event FROM write_audit WHERE session_id=?1 AND event IN ('session_end_remote_confirmed','session_end_remote_unconfirmed') ORDER BY sequence DESC LIMIT 1)", [id], |row| row.get(0))?;
        let remaining = db
            .prepare("SELECT operation,remaining FROM write_budgets WHERE session_id=?1")?
            .query_map([id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, u32>(1)?)))?
            .collect::<rusqlite::Result<BTreeMap<_, _>>>()?;
        let actions=db.prepare("SELECT id,operation,status,gateway_action_id,n8n_execution_id,gateway_audit_id FROM write_actions WHERE session_id=?1 ORDER BY rowid DESC LIMIT 100")?.query_map([id],|r|Ok(WriteActionStatus{id:r.get(0)?,operation:r.get(1)?,status:r.get(2)?,gateway_action_id:r.get(3)?,n8n_execution_id:r.get(4)?,gateway_audit_id:r.get(5)?}))?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(SessionStatus {
            id: id.into(),
            mode: match manifest {
                WriteManifest::SingleAction { .. } => "single_action",
                WriteManifest::BoundedSession { .. } => "bounded_session",
            }
            .into(),
            expires_at,
            revoked,
            gateway_end_confirmed: closure.map(|event| event == "session_end_remote_confirmed"),
            remaining,
            actions,
        })
    }
    pub(crate) fn record_closure_status(
        &self,
        id: &str,
        agent: AgentId,
        task: &str,
        confirmed: bool,
    ) -> Result<()> {
        let db = self
            .db
            .lock()
            .map_err(|_| anyhow::anyhow!("Write database lock poisoned"))?;
        let event = if confirmed {
            "session_end_remote_confirmed"
        } else {
            "session_end_remote_unconfirmed"
        };
        if db.execute("INSERT INTO write_audit(session_id,event,created_at) SELECT id,?1,?2 FROM write_sessions WHERE id=?3 AND agent=?4 AND task_id=?5 AND revoked=1", params![event, chrono::Utc::now().timestamp(), id, agent.as_str(), task])? != 1 {
            bail!("Closure evidence does not match a revoked session");
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_manifest_hash_matches_canonical_wire_json() {
        let canonical = r#"{"archive_mailbox":"Archive","budgets":{"email.move":2},"mailboxes":["Archive","INBOX"],"message_uids":{"INBOX":[1,2]},"mode":"bounded_session"}"#;
        let manifest = grant().manifest;
        assert_eq!(
            manifest.hash().unwrap(),
            hex::encode(Sha256::digest(canonical.as_bytes()))
        );
        assert_eq!(
            serde_json::to_string(&serde_json::to_value(manifest).unwrap()).unwrap(),
            canonical
        );
    }
    #[test]
    fn unknown_ledger_version_is_refused_before_schema_changes() {
        let directory = tempfile::TempDir::new().unwrap();
        let path = directory.path().join("future.db");
        let database = Connection::open(&path).unwrap();
        database.pragma_update(None, "user_version", 999).unwrap();
        assert!(WriteSessionStore::open(&path).is_err());
        let tables: i64 = database
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE name='write_sessions'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tables, 0);
    }
    #[test]
    fn legacy_audit_alias_migrates_once_without_restoring_budget() {
        let directory = tempfile::TempDir::new().unwrap();
        let path = directory.path().join("writes.db");
        let store = WriteSessionStore::open(&path).unwrap();
        store.record_verified(&grant()).unwrap();
        store
            .reserve(
                "session",
                "legacy-action",
                AgentId::Janet,
                "task",
                "broker",
                "email.move",
                &payload(1),
            )
            .unwrap();
        store
            .record_outcome(
                "legacy-action",
                WriteOutcome::Succeeded,
                Some("legacy-action"),
                Some("legacy-audit"),
                Some("execution"),
            )
            .unwrap();
        drop(store);
        let legacy = Connection::open(&path).unwrap();
        legacy
            .execute_batch(
                "UPDATE write_actions SET gateway_action_id=gateway_audit_id;
            ALTER TABLE write_actions DROP COLUMN gateway_audit_id; PRAGMA user_version=0;",
            )
            .unwrap();
        drop(legacy);
        for _ in 0..2 {
            let reopened = WriteSessionStore::open(&path).unwrap();
            let status = reopened.status("session", AgentId::Janet, "task").unwrap();
            assert_eq!(
                status.actions[0].gateway_action_id.as_deref(),
                Some("legacy-action")
            );
            assert_eq!(
                status.actions[0].gateway_audit_id.as_deref(),
                Some("legacy-audit")
            );
            assert_eq!(status.remaining["email.move"], 1);
            assert!(reopened
                .reserve(
                    "session",
                    "legacy-action",
                    AgentId::Janet,
                    "task",
                    "broker",
                    "email.move",
                    &payload(1)
                )
                .is_err());
            assert_eq!(
                reopened
                    .status("session", AgentId::Janet, "task")
                    .unwrap()
                    .remaining["email.move"],
                1
            );
        }
    }
    fn grant() -> VerifiedWriteGrant {
        let manifest = WriteManifest::BoundedSession {
            budgets: BTreeMap::from([("email.move".into(), 2)]),
            mailboxes: BTreeSet::from(["INBOX".into(), "Archive".into()]),
            message_uids: BTreeMap::from([("INBOX".into(), BTreeSet::from([1, 2]))]),
            archive_mailbox: "Archive".into(),
        };
        VerifiedWriteGrant {
            id: "session".into(),
            agent: AgentId::Janet,
            task_id: "task".into(),
            broker_identity: "broker".into(),
            reviewed_hash: manifest.hash().unwrap(),
            manifest,
            expires_at: chrono::Utc::now().timestamp() + 60,
            gateway_request_id: "request".into(),
            gateway_session_id: "gateway-session".into(),
            gateway_audit_id: "audit".into(),
            credential_reference: "safe-token-reference".into(),
        }
    }
    fn payload(uid: u32) -> serde_json::Value {
        serde_json::json!({"source_mailbox":"INBOX","destination_mailbox":"Archive","uid":uid})
    }
    #[test]
    fn concurrent_connections_cannot_exceed_reviewed_budget() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("write.db");
        let first = WriteSessionStore::open(&path).unwrap();
        let mut reviewed = grant();
        if let WriteManifest::BoundedSession { budgets, .. } = &mut reviewed.manifest {
            budgets.insert("email.move".into(), 1);
        }
        reviewed.reviewed_hash = reviewed.manifest.hash().unwrap();
        first.record_verified(&reviewed).unwrap();
        let second = WriteSessionStore::open(&path).unwrap();
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let attempts: Vec<_> = [first, second]
            .into_iter()
            .enumerate()
            .map(|(index, store)| {
                let barrier = std::sync::Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    store
                        .reserve(
                            "session",
                            &format!("action-{index}"),
                            AgentId::Janet,
                            "task",
                            "broker",
                            "email.move",
                            &payload(u32::try_from(index + 1).unwrap()),
                        )
                        .is_ok()
                })
            })
            .collect();
        let admitted = attempts
            .into_iter()
            .filter_map(|attempt| attempt.join().unwrap().then_some(()))
            .count();
        assert_eq!(admitted, 1);
        let reopened = WriteSessionStore::open(&path).unwrap();
        let state = reopened.status("session", AgentId::Janet, "task").unwrap();
        assert_eq!(state.remaining["email.move"], 0);
        assert_eq!(state.actions.len(), 1);
        assert_eq!(state.actions[0].status, "reserved");
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn resource_owner_broker_budget_and_replay_are_enforced() {
        let d = tempfile::TempDir::new().unwrap();
        let store = WriteSessionStore::open(&d.path().join("write.db")).unwrap();
        store.record_verified(&grant()).unwrap();
        assert!(store
            .reserve(
                "session",
                "a",
                AgentId::Harley,
                "task",
                "broker",
                "email.move",
                &payload(1)
            )
            .is_err());
        assert!(store
            .reserve(
                "session",
                "a",
                AgentId::Janet,
                "wrong",
                "broker",
                "email.move",
                &payload(1)
            )
            .is_err());
        assert!(store
            .reserve(
                "session",
                "a",
                AgentId::Janet,
                "task",
                "wrong",
                "email.move",
                &payload(1)
            )
            .is_err());
        assert!(store
            .reserve(
                "session",
                "a",
                AgentId::Janet,
                "task",
                "broker",
                "email.move",
                &payload(3)
            )
            .is_err());
        store
            .reserve(
                "session",
                "a",
                AgentId::Janet,
                "task",
                "broker",
                "email.move",
                &payload(1),
            )
            .unwrap();
        assert!(store
            .reserve(
                "session",
                "a",
                AgentId::Janet,
                "task",
                "broker",
                "email.move",
                &payload(1)
            )
            .is_err());
        store
            .reserve(
                "session",
                "b",
                AgentId::Janet,
                "task",
                "broker",
                "email.move",
                &payload(2),
            )
            .unwrap();
        assert_eq!(
            store
                .status("session", AgentId::Janet, "task")
                .unwrap()
                .remaining["email.move"],
            0
        );
        assert!(store
            .reserve(
                "session",
                "c",
                AgentId::Janet,
                "task",
                "broker",
                "email.move",
                &payload(2)
            )
            .is_err());
        store.end("session", AgentId::Janet, "task").unwrap();
        store
            .record_outcome(
                "a",
                WriteOutcome::Succeeded,
                Some("a"),
                Some("audit-a"),
                Some("execution-a"),
            )
            .unwrap();
        assert!(
            store
                .status("session", AgentId::Janet, "task")
                .unwrap()
                .revoked
        );
    }
    #[test]
    fn restart_uncertainty_requires_evidence_and_never_refunds_budget() {
        let d = tempfile::TempDir::new().unwrap();
        let path = d.path().join("write.db");
        let s = WriteSessionStore::open(&path).unwrap();
        s.record_verified(&grant()).unwrap();
        s.reserve(
            "session",
            "a",
            AgentId::Janet,
            "task",
            "broker",
            "email.move",
            &payload(1),
        )
        .unwrap();
        s.db.lock()
            .unwrap()
            .execute(
                "UPDATE write_actions SET lease_expires_at=0 WHERE id='a'",
                [],
            )
            .unwrap();
        drop(s);
        let s = WriteSessionStore::open(&path).unwrap();
        assert!(s
            .reserve(
                "session",
                "b",
                AgentId::Janet,
                "task",
                "broker",
                "email.move",
                &payload(1)
            )
            .is_err());
        assert!(s.reconcile("a", false, "").is_err());
        s.reconcile("a", false, "archive/read-result-1").unwrap();
        s.reserve(
            "session",
            "b",
            AgentId::Janet,
            "task",
            "broker",
            "email.move",
            &payload(1),
        )
        .unwrap();
        assert_eq!(
            s.status("session", AgentId::Janet, "task")
                .unwrap()
                .remaining["email.move"],
            0
        );
    }
    #[test]
    fn exact_email_payload_changes_expiry_and_revoke_deny() {
        let d = tempfile::TempDir::new().unwrap();
        let s = WriteSessionStore::open(&d.path().join("write.db")).unwrap();
        let mut g = grant();
        g.manifest = WriteManifest::SingleAction {
            operation: "email.send".into(),
            payload: serde_json::json!({"to":["example@example.com"],"body":"Reviewed"}),
        };
        g.reviewed_hash = g.manifest.hash().unwrap();
        s.record_verified(&g).unwrap();
        assert!(s
            .reserve(
                "session",
                "a",
                AgentId::Janet,
                "task",
                "broker",
                "email.send",
                &serde_json::json!({"to":["example@example.com"],"body":"Changed"})
            )
            .is_err());
        s.end("session", AgentId::Janet, "task").unwrap();
        let WriteManifest::SingleAction { payload, .. } = &g.manifest else {
            unreachable!()
        };
        assert!(s
            .reserve(
                "session",
                "a",
                AgentId::Janet,
                "task",
                "broker",
                "email.send",
                payload
            )
            .is_err());
        g.id = "expired".into();
        g.expires_at = 0;
        assert!(s.record_verified(&g).is_err());
    }
    #[test]
    fn invalid_gateway_scope_cannot_be_registered() {
        let d = tempfile::TempDir::new().unwrap();
        let s = WriteSessionStore::open(&d.path().join("write.db")).unwrap();
        let mut g = grant();
        g.reviewed_hash = "tampered".into();
        assert!(s.record_verified(&g).is_err());
        g = grant();
        g.expires_at = chrono::Utc::now().timestamp() + 3600;
        assert!(s.record_verified(&g).is_err());
    }
}
