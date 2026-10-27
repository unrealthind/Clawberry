//! Security admission primitives for permanent-agent execution.
//!
//! This module is intentionally independent from the legacy tool registry.  It is
//! the only admission path that a future Janet/Harley execution broker may use;
//! creating this boundary first prevents a tool integration from becoming an
//! unreviewed bypass.

use crate::assistants::AgentId;
use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::sync::Mutex;
use uuid::Uuid;

/// Operations that cannot be approved by this product, even by an administrator
/// using the agent UI.  Host administration is deliberately outside this API.
pub const ROOT_DENY_OPERATIONS: &[&str] = &[
    "security.disable_guard",
    "security.bypass_policy",
    "security.modify_root_policy",
    "security.export_raw_secrets",
    "security.tamper_audit",
    "security.tamper_security_db",
    "security.grant_host_root",
    "security.self_authorize",
    "agent.read_private_memory",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionRequest {
    pub id: String,
    pub agent: AgentId,
    pub task_id: String,
    pub capability: String,
    pub operation: String,
    pub resource: String,
    /// Canonical payload, retained only as a SHA-256 binding in approval state.
    pub payload: serde_json::Value,
}

impl ExecutionRequest {
    pub fn new(
        agent: AgentId,
        task_id: impl Into<String>,
        capability: impl Into<String>,
        operation: impl Into<String>,
        resource: impl Into<String>,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            agent,
            task_id: task_id.into(),
            capability: capability.into(),
            operation: operation.into(),
            resource: resource.into(),
            payload,
        }
    }
    pub fn payload_hash(&self) -> Result<String> {
        Ok(hex::encode(Sha256::digest(serde_json::to_vec(
            &self.payload,
        )?)))
    }
    pub fn validate(&self) -> Result<()> {
        if !Uuid::parse_str(&self.id).is_ok_and(|parsed| parsed.to_string() == self.id)
            || [
                (&self.task_id, 256),
                (&self.capability, 128),
                (&self.operation, 128),
                (&self.resource, 512),
            ]
            .iter()
            .any(|(value, limit)| {
                value.trim().is_empty()
                    || value.len() > *limit
                    || value.chars().any(char::is_control)
            })
            || serde_json::to_vec(self)?.len() > 32_000
        {
            bail!("Execution request has invalid or oversized scope");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuardDecision {
    Safe,
    Block,
    Unavailable,
}

/// A deterministic adapter boundary so the live Llama Guard client can be added
/// without placing its availability or prompt parsing in the policy engine.
pub trait Guard: Send + Sync {
    fn assess(&self, request: &ExecutionRequest) -> GuardDecision;
}

#[derive(Debug, Clone, Copy)]
pub struct FailClosedGuard;
impl Guard for FailClosedGuard {
    fn assess(&self, _: &ExecutionRequest) -> GuardDecision {
        GuardDecision::Unavailable
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Admission {
    Allowed { approval_id: String },
    AllowedReadByPolicy,
    Denied { reason: String },
    NeedsDeepReview { reason: String },
}

#[derive(Debug, Clone)]
pub struct ApprovalGrant {
    pub id: String,
    pub agent: AgentId,
    pub task_id: String,
    pub capability: String,
    pub operation: String,
    pub resource: String,
    pub payload_hash: String,
    pub expires_at: DateTime<Utc>,
    pub remaining_uses: u32,
}

/// Durable, atomic single-action approval store.  Session/TOTP grants build on
/// this shape in P11/P12; this type intentionally cannot issue its own approval.
pub struct ApprovalStore {
    db: Mutex<Connection>,
}
#[derive(Debug)]
pub(crate) struct GuardBlockedAction(pub ExecutionRequest);
impl std::fmt::Display for GuardBlockedAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Guard blocked action; deep review required")
    }
}
impl std::error::Error for GuardBlockedAction {}
impl ApprovalStore {
    pub fn open(path: &Path) -> Result<Self> {
        if path
            .symlink_metadata()
            .is_ok_and(|m| !m.is_file() || m.file_type().is_symlink())
        {
            bail!("Approval storage must be a regular non-symlink file");
        }
        if !path.exists() {
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            options.open(path)?;
        }
        let db = Connection::open(path)?;
        db.busy_timeout(std::time::Duration::from_secs(5))?;
        db.execute_batch(
            "PRAGMA foreign_keys=ON; CREATE TABLE IF NOT EXISTS execution_approvals (
          id TEXT PRIMARY KEY, agent TEXT NOT NULL, task_id TEXT NOT NULL, capability TEXT NOT NULL,
          operation TEXT NOT NULL, resource TEXT NOT NULL, payload_hash TEXT NOT NULL,
          expires_at TEXT NOT NULL, remaining_uses INTEGER NOT NULL CHECK(remaining_uses >= 0),
          created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);
          CREATE TABLE IF NOT EXISTS execution_audit (
          sequence INTEGER PRIMARY KEY AUTOINCREMENT, request_id TEXT NOT NULL,
          agent TEXT NOT NULL, task_id TEXT NOT NULL, operation TEXT NOT NULL,
          payload_hash TEXT NOT NULL, decision TEXT NOT NULL, approval_id TEXT,
          created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);
          CREATE UNIQUE INDEX IF NOT EXISTS admitted_request_once ON execution_audit(request_id) WHERE decision='admitted';",
        )?;
        db.execute_batch("CREATE TABLE IF NOT EXISTS guard_exception_reviews(id TEXT PRIMARY KEY,request_hash TEXT NOT NULL,reason TEXT NOT NULL,expires_at TEXT NOT NULL); CREATE TABLE IF NOT EXISTS guard_exception_grants(approval_id TEXT PRIMARY KEY REFERENCES execution_approvals(id),request_hash TEXT NOT NULL);")?;
        Ok(Self { db: Mutex::new(db) })
    }
    pub(crate) fn record_exception_review(
        &self,
        request: &ExecutionRequest,
        reason: &str,
    ) -> Result<String> {
        if ROOT_DENY_OPERATIONS.contains(&request.operation.as_str())
            || reason.trim().is_empty()
            || reason.len() > 4000
        {
            bail!("Invalid exception review")
        }
        let id = Uuid::new_v4().to_string();
        let hash = hex::encode(Sha256::digest(serde_json::to_vec(request)?));
        self.db
            .lock()
            .map_err(|_| anyhow::anyhow!("Approval database lock poisoned"))?
            .execute(
                "INSERT INTO guard_exception_reviews VALUES(?1,?2,?3,?4)",
                params![
                    id,
                    hash,
                    reason,
                    (Utc::now() + chrono::Duration::minutes(5)).to_rfc3339()
                ],
            )?;
        Ok(id)
    }
    pub(crate) fn approve_exception(
        &self,
        request: &ExecutionRequest,
        review_id: &str,
    ) -> Result<String> {
        if ROOT_DENY_OPERATIONS.contains(&request.operation.as_str()) {
            bail!("HARD DENY")
        }
        let hash = hex::encode(Sha256::digest(serde_json::to_vec(request)?));
        let mut db = self
            .db
            .lock()
            .map_err(|_| anyhow::anyhow!("Approval database lock poisoned"))?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let expires:String=tx.query_row("SELECT expires_at FROM guard_exception_reviews WHERE id=?1 AND request_hash=?2 AND expires_at>?3",params![review_id,hash,Utc::now().to_rfc3339()],|r|r.get(0))?;
        let id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO execution_approvals(id,agent,task_id,capability,operation,resource,payload_hash,expires_at,remaining_uses) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,1)",params![id,request.agent.as_str(),request.task_id,request.capability,request.operation,request.resource,request.payload_hash()?,expires])?;
        tx.execute(
            "INSERT INTO guard_exception_grants VALUES(?1,?2)",
            params![id, hash],
        )?;
        tx.execute(
            "DELETE FROM guard_exception_reviews WHERE id=?1",
            [review_id],
        )?;
        tx.commit()?;
        Ok(id)
    }
    pub fn revoke(&self, id: &str, agent: AgentId, task: &str) -> Result<()> {
        let db = self
            .db
            .lock()
            .map_err(|_| anyhow::anyhow!("Approval database lock poisoned"))?;
        if db.execute("UPDATE execution_approvals SET remaining_uses=0 WHERE id=?1 AND agent=?2 AND task_id=?3", params![id,agent.as_str(),task])? != 1 {
            bail!("Approval not found in owner/task scope");
        }
        Ok(())
    }
    pub fn record_human_grant(&self, grant: &ApprovalGrant) -> Result<()> {
        if grant.remaining_uses != 1 || grant.expires_at <= Utc::now() {
            bail!("Single-action approval requires one execution and a future expiry");
        }
        if [
            &grant.id,
            &grant.task_id,
            &grant.capability,
            &grant.operation,
            &grant.resource,
        ]
        .iter()
        .any(|s| s.trim().is_empty())
            || grant.payload_hash.len() != 64
            || !grant.payload_hash.bytes().all(|b| b.is_ascii_hexdigit())
            || ROOT_DENY_OPERATIONS.contains(&grant.operation.as_str())
            || grant.operation == "n8n.write"
        {
            bail!("Invalid or root-denied approval scope");
        }
        self.db.lock().map_err(|_| anyhow::anyhow!("Approval database lock poisoned"))?.execute(
            "INSERT INTO execution_approvals(id,agent,task_id,capability,operation,resource,payload_hash,expires_at,remaining_uses) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![grant.id, grant.agent.as_str(), grant.task_id, grant.capability, grant.operation, grant.resource, grant.payload_hash, grant.expires_at.to_rfc3339(), grant.remaining_uses])?;
        Ok(())
    }
    fn consume_matching(
        &self,
        request: &ExecutionRequest,
        exception: bool,
    ) -> Result<Option<String>> {
        let hash = request.payload_hash()?;
        let request_hash = hex::encode(Sha256::digest(serde_json::to_vec(request)?));
        let mut db = self
            .db
            .lock()
            .map_err(|_| anyhow::anyhow!("Approval database lock poisoned"))?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let replay: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM execution_audit WHERE request_id=?1 AND decision='admitted')", [&request.id], |row| row.get(0))?;
        if replay {
            return Ok(None);
        }
        let candidate: Option<String> = tx.query_row(
          "SELECT a.id FROM execution_approvals a LEFT JOIN guard_exception_grants g ON g.approval_id=a.id WHERE agent=?1 AND task_id=?2 AND capability=?3 AND operation=?4 AND resource=?5 AND payload_hash=?6 AND expires_at>?7 AND remaining_uses>0 AND ((?8=0 AND g.approval_id IS NULL) OR (?8=1 AND g.request_hash=?9)) ORDER BY created_at LIMIT 1",
          params![request.agent.as_str(),request.task_id,request.capability,request.operation,request.resource,hash,Utc::now().to_rfc3339(),exception,request_hash], |r| r.get(0)).optional()?;
        if let Some(id) = candidate {
            tx.execute("UPDATE execution_approvals SET remaining_uses=remaining_uses-1 WHERE id=?1 AND remaining_uses>0", [&id])?;
            tx.execute("INSERT INTO execution_audit(request_id,agent,task_id,operation,payload_hash,decision,approval_id) VALUES(?1,?2,?3,?4,?5,'admitted',?6)",params![request.id,request.agent.as_str(),request.task_id,request.operation,hash,id])?;
            tx.commit()?;
            Ok(Some(id))
        } else {
            tx.commit()?;
            Ok(None)
        }
    }

    fn reserve_autonomous_read(&self, request: &ExecutionRequest) -> Result<bool> {
        let mut db = self
            .db
            .lock()
            .map_err(|_| anyhow::anyhow!("Approval database lock poisoned"))?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let inserted = tx.execute(
            "INSERT INTO execution_audit(request_id,agent,task_id,operation,payload_hash,decision) SELECT ?1,?2,?3,?4,?5,'admitted' WHERE NOT EXISTS(SELECT 1 FROM execution_audit WHERE request_id=?1 AND decision='admitted')",
            params![request.id,request.agent.as_str(),request.task_id,request.operation,request.payload_hash()?]
        )?;
        tx.commit()?;
        Ok(inserted == 1)
    }

    fn record_denial(&self, request: &ExecutionRequest, decision: &str) -> Result<()> {
        self.db.lock().map_err(|_| anyhow::anyhow!("Approval database lock poisoned"))?.execute(
            "INSERT INTO execution_audit(request_id,agent,task_id,operation,payload_hash,decision) VALUES(?1,?2,?3,?4,?5,?6)",
            params![request.id,request.agent.as_str(),request.task_id,request.operation,request.payload_hash()?,decision])?;
        Ok(())
    }
}

pub struct ExecutionBroker<G> {
    approvals: ApprovalStore,
    guard: G,
    policy: super::root::RootPolicy,
}
impl<G: Guard> ExecutionBroker<G> {
    /// Admission for the separate TOTP ledger. This checks policy/Guard only;
    /// the trusted write adapter must atomically reserve a verified grant next.
    pub(crate) fn check_write(&self, request: &ExecutionRequest) -> Result<()> {
        if request.operation != "n8n.write"
            || request.capability != "n8n.write"
            || request.validate().is_err()
            || !self.policy.allows(request)
        {
            self.approvals
                .record_denial(request, "write_policy_denied")?;
            bail!("Write denied by host policy");
        }
        let decision = self.guard.assess(request);
        self.approvals.record_denial(
            request,
            match decision {
                GuardDecision::Safe => "write_guard_safe",
                GuardDecision::Block => "write_guard_block",
                GuardDecision::Unavailable => "write_guard_unavailable",
            },
        )?;
        if decision == GuardDecision::Block {
            if self.approvals.consume_matching(request, true)?.is_some() {
                return Ok(());
            }
            return Err(GuardBlockedAction(request.clone()).into());
        }
        if decision != GuardDecision::Safe {
            bail!("Write blocked by Guard; no dispatch or budget consumption")
        }
        Ok(())
    }
    pub fn new(approvals: ApprovalStore, guard: G) -> Self {
        Self {
            approvals,
            guard,
            policy: super::root::RootPolicy::default(),
        }
    }
    pub fn with_policy(
        approvals: ApprovalStore,
        guard: G,
        policy: super::root::RootPolicy,
    ) -> Self {
        Self {
            approvals,
            guard,
            policy,
        }
    }
    pub fn admit(&self, request: &ExecutionRequest) -> Result<Admission> {
        let result = self.assess(request)?;
        match &result {
            Admission::Allowed { .. } => {} // admission and consumption share a transaction
            Admission::AllowedReadByPolicy => self
                .approvals
                .record_denial(request, "policy_read_admitted")?,
            Admission::Denied { .. } => self.approvals.record_denial(request, "denied")?,
            Admission::NeedsDeepReview { .. } => {
                self.approvals.record_denial(request, "guard_block")?;
            }
        }
        Ok(result)
    }

    fn assess(&self, request: &ExecutionRequest) -> Result<Admission> {
        if ROOT_DENY_OPERATIONS.contains(&request.operation.as_str()) {
            return Ok(Admission::Denied {
                reason: "HARD DENY: root-of-trust operation requires manual host administration"
                    .into(),
            });
        }
        if request.validate().is_err() {
            return Ok(Admission::Denied {
                reason: "Execution scope is invalid or exceeds its limits".into(),
            });
        }
        if !self.policy.allows(request) {
            return Ok(Admission::Denied {
                reason: "Operation is not granted by the host-owned policy".into(),
            });
        }
        match self.guard.assess(request) {
            GuardDecision::Safe => {},
            GuardDecision::Block => return Ok(match self.approvals.consume_matching(request,true)? {Some(approval_id)=>Admission::Allowed{approval_id},None=>Admission::NeedsDeepReview { reason: "Guard blocked the request; deep review and exact human exception approval required".into() }}),
            GuardDecision::Unavailable => return Ok(Admission::Denied { reason: "Guard is unavailable; execution fails closed".into() }),
        }
        if self.policy.permits_autonomous_read(request) {
            return Ok(if self.approvals.reserve_autonomous_read(request)? {
                Admission::AllowedReadByPolicy
            } else {
                Admission::Denied {
                    reason: "Request identity was already admitted; replay denied".into(),
                }
            });
        }
        Ok(match self.approvals.consume_matching(request,false)? { Some(approval_id) => Admission::Allowed { approval_id }, None => Admission::Denied { reason: "No unexpired human approval binds this exact agent, task, operation, resource, and payload".into() } })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    #[derive(Clone, Copy)]
    struct TestGuard(GuardDecision);
    impl Guard for TestGuard {
        fn assess(&self, _: &ExecutionRequest) -> GuardDecision {
            self.0
        }
    }
    fn request() -> ExecutionRequest {
        ExecutionRequest::new(
            AgentId::Janet,
            "task-1",
            "shell",
            "shell.execute",
            "workspace/task-1",
            serde_json::json!({"command":"git status"}),
        )
    }
    fn store(dir: &TempDir) -> ApprovalStore {
        ApprovalStore::open(&dir.path().join("approvals.db")).unwrap()
    }
    fn broker(d: &TempDir, guard: impl Guard) -> ExecutionBroker<impl Guard> {
        ExecutionBroker::with_policy(
            store(d),
            guard,
            super::super::root::RootPolicy::fixture(
                AgentId::Janet,
                "shell",
                "shell.execute",
                "workspace/task-1",
            ),
        )
    }
    #[test]
    fn invalid_scope_never_reaches_guard_or_spends_approval() {
        struct CountingGuard(std::sync::Arc<std::sync::atomic::AtomicUsize>);
        impl Guard for CountingGuard {
            fn assess(&self, _: &ExecutionRequest) -> GuardDecision {
                self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                GuardDecision::Safe
            }
        }
        let directory = TempDir::new().unwrap();
        let original = request();
        store(&directory)
            .record_human_grant(&grant(&original))
            .unwrap();
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let broker = broker(&directory, CountingGuard(calls.clone()));
        let mut invalid = original.clone();
        for identity in ["not-a-uuid".to_string(), original.id.replace('-', "")] {
            invalid.id = identity;
            assert!(matches!(
                broker.admit(&invalid).unwrap(),
                Admission::Denied { .. }
            ));
        }
        invalid = original.clone();
        invalid.payload = serde_json::json!({"command":"x".repeat(32_000)});
        assert!(matches!(
            broker.admit(&invalid).unwrap(),
            Admission::Denied { .. }
        ));
        invalid = original.clone();
        invalid.task_id.push('\n');
        assert!(matches!(
            broker.admit(&invalid).unwrap(),
            Admission::Denied { .. }
        ));
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
        assert!(matches!(
            broker.admit(&original).unwrap(),
            Admission::Allowed { .. }
        ));
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    }
    #[test]
    fn safer_alternative_reenters_guard_and_requires_its_own_approval() {
        struct ReformulationGuard;
        impl Guard for ReformulationGuard {
            fn assess(&self, request: &ExecutionRequest) -> GuardDecision {
                if request.payload["command"] == "printf reviewed" {
                    GuardDecision::Safe
                } else {
                    GuardDecision::Block
                }
            }
        }
        let directory = TempDir::new().unwrap();
        let original = request();
        store(&directory)
            .record_human_grant(&grant(&original))
            .unwrap();
        let broker = broker(&directory, ReformulationGuard);
        assert!(matches!(
            broker.admit(&original).unwrap(),
            Admission::NeedsDeepReview { .. }
        ));
        let mut alternative = original.clone();
        alternative.id = Uuid::new_v4().to_string();
        alternative.payload = serde_json::json!({"command":"printf reviewed"});
        let review = super::super::guard::DeepReview::SaferAlternative {
            reason: "Use the bounded fixture command".into(),
            request: alternative.clone(),
        };
        review.alternative(&original).unwrap().unwrap();
        assert!(matches!(
            broker.admit(&alternative).unwrap(),
            Admission::Denied { .. }
        ));
        store(&directory)
            .record_human_grant(&grant(&alternative))
            .unwrap();
        assert!(matches!(
            broker.admit(&alternative).unwrap(),
            Admission::Allowed { .. }
        ));
        assert!(matches!(
            broker.admit(&alternative).unwrap(),
            Admission::Denied { .. }
        ));
        assert!(matches!(
            broker.admit(&original).unwrap(),
            Admission::NeedsDeepReview { .. }
        ));
    }

    #[tokio::test]
    #[ignore = "Requires rootless Podman and administrator-built digest-pinned image"]
    async fn real_sandbox_dispatch_requires_single_use_broker_admission() {
        let directory = TempDir::new().unwrap();
        let mut action = request();
        action.payload = serde_json::json!({"command":"printf approved-sandbox-fixture"});
        store(&directory)
            .record_human_grant(&grant(&action))
            .unwrap();
        let broker = broker(&directory, TestGuard(GuardDecision::Safe));
        assert!(matches!(
            broker.admit(&action).unwrap(),
            Admission::Allowed { .. }
        ));
        let sandbox = crate::sandbox::SandboxConfig {
            image: std::env::var("RAAVICLAW_SANDBOX_IMAGE").expect("Set a reviewed image digest"),
            timeout_secs: 15,
            memory_mb: 256,
            cpus: 1,
        };
        let result = sandbox
            .execute(
                directory.path(),
                action.payload["command"].as_str().unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(result.exit_code, Some(0), "{}", result.stderr);
        assert_eq!(result.stdout, "approved-sandbox-fixture");
        assert!(matches!(
            broker.admit(&action).unwrap(),
            Admission::Denied { .. }
        ));
        drop(broker);
        assert!(matches!(
            self::broker(&directory, TestGuard(GuardDecision::Safe))
                .admit(&action)
                .unwrap(),
            Admission::Denied { .. }
        ));
    }
    #[test]
    fn review_cannot_override_guard_but_exact_human_exception_can() {
        let d = TempDir::new().unwrap();
        let r = request();
        let s = store(&d);
        let review = s
            .record_exception_review(&r, "Scoped investigation is justified")
            .unwrap();
        let b = ExecutionBroker::with_policy(
            s,
            TestGuard(GuardDecision::Block),
            super::super::root::RootPolicy::fixture(
                AgentId::Janet,
                "shell",
                "shell.execute",
                "workspace/task-1",
            ),
        );
        assert!(matches!(
            b.admit(&r).unwrap(),
            Admission::NeedsDeepReview { .. }
        ));
        let mut changed = r.clone();
        changed.payload = serde_json::json!({"command":"different"});
        assert!(b.approvals.approve_exception(&changed, &review).is_err());
        b.approvals.approve_exception(&r, &review).unwrap();
        assert!(b.approvals.approve_exception(&r, &review).is_err());
        assert!(matches!(b.admit(&r).unwrap(), Admission::Allowed { .. }));
        assert!(matches!(
            b.admit(&r).unwrap(),
            Admission::NeedsDeepReview { .. }
        ));
    }
    #[test]
    fn safe_autonomous_read_cannot_replay_a_consumed_exception() {
        let d = TempDir::new().unwrap();
        let mut request = request();
        request.capability = "n8n.read".into();
        request.operation = "n8n.read".into();
        request.resource = "email_readonly".into();
        let approvals = store(&d);
        let review = approvals
            .record_exception_review(&request, "Scoped read")
            .unwrap();
        approvals.approve_exception(&request, &review).unwrap();
        assert!(approvals
            .consume_matching(&request, true)
            .unwrap()
            .is_some());
        drop(approvals);
        let broker = ExecutionBroker::with_policy(
            store(&d),
            TestGuard(GuardDecision::Safe),
            super::super::root::RootPolicy::read_fixture(AgentId::Janet, "email_readonly"),
        );
        assert!(matches!(
            broker.admit(&request).unwrap(),
            Admission::Denied { .. }
        ));
        request.id = uuid::Uuid::new_v4().to_string();
        assert!(matches!(
            broker.admit(&request).unwrap(),
            Admission::AllowedReadByPolicy
        ));
        assert!(matches!(
            broker.admit(&request).unwrap(),
            Admission::Denied { .. }
        ));
    }

    #[test]
    fn exact_exception_survives_restart_and_consumption_stays_spent() {
        let d = TempDir::new().unwrap();
        let request = request();
        let review = {
            let approvals = store(&d);
            approvals
                .record_exception_review(&request, "Read acceptance")
                .unwrap()
        };
        {
            let approvals = store(&d);
            approvals.approve_exception(&request, &review).unwrap();
        }
        {
            let approvals = store(&d);
            assert!(approvals
                .consume_matching(&request, true)
                .unwrap()
                .is_some());
        }
        let reopened = store(&d);
        assert!(reopened.consume_matching(&request, true).unwrap().is_none());
        assert!(reopened.approve_exception(&request, &review).is_err());
        let db = reopened.db.lock().unwrap();
        let admitted: u64 = db.query_row(
            "SELECT COUNT(*) FROM execution_audit WHERE request_id=?1 AND decision='admitted' AND approval_id IS NOT NULL",
            [&request.id], |row| row.get(0)
        ).unwrap();
        assert_eq!(admitted, 1);
    }

    #[test]
    fn unavailable_guard_and_root_denial_cannot_use_exceptions() {
        let d = TempDir::new().unwrap();
        let mut r = request();
        let s = store(&d);
        let review = s.record_exception_review(&r, "Review reason").unwrap();
        s.approve_exception(&r, &review).unwrap();
        let b = ExecutionBroker::with_policy(
            s,
            FailClosedGuard,
            super::super::root::RootPolicy::fixture(
                AgentId::Janet,
                "shell",
                "shell.execute",
                "workspace/task-1",
            ),
        );
        assert!(matches!(b.admit(&r).unwrap(), Admission::Denied { .. }));
        r.operation = "security.disable_guard".into();
        assert!(b.approvals.record_exception_review(&r, "reason").is_err());
        assert!(b.approvals.approve_exception(&r, &review).is_err());
    }
    #[test]
    fn revocation_and_non_totp_write_grants_fail_closed() {
        let d = TempDir::new().unwrap();
        let r = request();
        let s = store(&d);
        let g = grant(&r);
        s.record_human_grant(&g).unwrap();
        assert!(s.revoke(&g.id, AgentId::Harley, &r.task_id).is_err());
        s.revoke(&g.id, r.agent, &r.task_id).unwrap();
        assert!(s.consume_matching(&r, false).unwrap().is_none());
        let mut g = grant(&r);
        g.operation = "n8n.write".into();
        assert!(s.record_human_grant(&g).is_err());
    }
    #[test]
    fn root_denial_precedes_guard_and_approval() {
        let d = TempDir::new().unwrap();
        let mut r = request();
        r.operation = "security.disable_guard".into();
        let out = ExecutionBroker::new(store(&d), TestGuard(GuardDecision::Safe))
            .admit(&r)
            .unwrap();
        assert!(matches!(out, Admission::Denied { .. }));
    }
    #[test]
    fn guard_failure_fails_closed() {
        let d = TempDir::new().unwrap();
        let out = broker(&d, FailClosedGuard).admit(&request()).unwrap();
        assert!(matches!(out, Admission::Denied { .. }));
    }
    #[test]
    fn blocked_guard_needs_review_not_execution() {
        let d = TempDir::new().unwrap();
        let out = broker(&d, TestGuard(GuardDecision::Block))
            .admit(&request())
            .unwrap();
        assert!(matches!(out, Admission::NeedsDeepReview { .. }));
    }
    #[test]
    fn exact_grant_is_single_use_and_payload_bound() {
        let d = TempDir::new().unwrap();
        let r = request();
        let s = store(&d);
        s.record_human_grant(&ApprovalGrant {
            id: "a1".into(),
            agent: AgentId::Janet,
            task_id: "task-1".into(),
            capability: "shell".into(),
            operation: "shell.execute".into(),
            resource: "workspace/task-1".into(),
            payload_hash: r.payload_hash().unwrap(),
            expires_at: Utc::now() + chrono::Duration::minutes(1),
            remaining_uses: 1,
        })
        .unwrap();
        let b = ExecutionBroker::with_policy(
            s,
            TestGuard(GuardDecision::Safe),
            super::super::root::RootPolicy::fixture(
                AgentId::Janet,
                "shell",
                "shell.execute",
                "workspace/task-1",
            ),
        );
        assert!(matches!(b.admit(&r).unwrap(), Admission::Allowed { .. }));
        assert!(matches!(b.admit(&r).unwrap(), Admission::Denied { .. }));
    }

    fn grant(r: &ExecutionRequest) -> ApprovalGrant {
        ApprovalGrant {
            id: Uuid::new_v4().to_string(),
            agent: r.agent,
            task_id: r.task_id.clone(),
            capability: r.capability.clone(),
            operation: r.operation.clone(),
            resource: r.resource.clone(),
            payload_hash: r.payload_hash().unwrap(),
            expires_at: Utc::now() + chrono::Duration::minutes(1),
            remaining_uses: 1,
        }
    }
    #[test]
    fn mismatched_scope_never_consumes_grant() {
        let d = TempDir::new().unwrap();
        let r = request();
        let s = store(&d);
        s.record_human_grant(&grant(&r)).unwrap();
        let b = ExecutionBroker::with_policy(
            s,
            TestGuard(GuardDecision::Safe),
            super::super::root::RootPolicy::fixture(
                AgentId::Janet,
                "shell",
                "shell.execute",
                "workspace/task-1",
            ),
        );
        let mut altered = r.clone();
        altered.payload = serde_json::json!({"command":"different command"});
        assert!(matches!(
            b.admit(&altered).unwrap(),
            Admission::Denied { .. }
        ));
        altered = r.clone();
        altered.task_id = "task-2".into();
        assert!(matches!(
            b.admit(&altered).unwrap(),
            Admission::Denied { .. }
        ));
        assert!(matches!(b.admit(&r).unwrap(), Admission::Allowed { .. }));
    }
    #[test]
    fn concurrent_process_connections_cannot_double_consume() {
        let d = TempDir::new().unwrap();
        let r = request();
        let s = store(&d);
        s.record_human_grant(&grant(&r)).unwrap();
        drop(s);
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
        let joins: Vec<_> = (0..8)
            .map(|_| {
                let path = d.path().join("approvals.db");
                let r = r.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    let b = ExecutionBroker::with_policy(
                        ApprovalStore::open(&path).unwrap(),
                        TestGuard(GuardDecision::Safe),
                        super::super::root::RootPolicy::fixture(
                            AgentId::Janet,
                            "shell",
                            "shell.execute",
                            "workspace/task-1",
                        ),
                    );
                    barrier.wait();
                    matches!(b.admit(&r).unwrap(), Admission::Allowed { .. })
                })
            })
            .collect();
        assert_eq!(
            joins
                .into_iter()
                .map(|join| usize::from(join.join().unwrap()))
                .sum::<usize>(),
            1
        );
    }
    #[test]
    fn root_grants_reusable_grants_and_expired_grants_are_rejected() {
        let d = TempDir::new().unwrap();
        let s = store(&d);
        let mut g = grant(&request());
        g.remaining_uses = 2;
        assert!(s.record_human_grant(&g).is_err());
        g.remaining_uses = 1;
        g.expires_at = Utc::now() - chrono::Duration::seconds(1);
        assert!(s.record_human_grant(&g).is_err());
        g.expires_at = Utc::now() + chrono::Duration::minutes(1);
        g.operation = "security.disable_guard".into();
        assert!(s.record_human_grant(&g).is_err());
    }
    #[test]
    fn admitted_request_identity_is_not_replayable_with_another_grant() {
        let d = TempDir::new().unwrap();
        let r = request();
        let s = store(&d);
        s.record_human_grant(&grant(&r)).unwrap();
        s.record_human_grant(&grant(&r)).unwrap();
        let b = ExecutionBroker::with_policy(
            s,
            TestGuard(GuardDecision::Safe),
            super::super::root::RootPolicy::fixture(
                AgentId::Janet,
                "shell",
                "shell.execute",
                "workspace/task-1",
            ),
        );
        assert!(matches!(b.admit(&r).unwrap(), Admission::Allowed { .. }));
        assert!(matches!(b.admit(&r).unwrap(), Admission::Denied { .. }));
    }
}
