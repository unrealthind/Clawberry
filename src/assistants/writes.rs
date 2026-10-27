//! Dedicated human TOTP handoff. Codes are never accepted by the operation/chat APIs.
//! The administrator adapter contract requires exact manifest echo and gateway replay protection.
use super::{AgentHost, AgentId};
use crate::security::write_sessions::{
    VerifiedWriteGrant, WriteManifest, WriteOutcome, WriteSessionStore,
};
use crate::security::{ExecutionRequest, SecretBroker};
use anyhow::{bail, Result};
use rusqlite::{params, Connection, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::io::Read;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WriteWorkflow {
    pub name: String,
    pub endpoint: String,
    pub credential: String,
    pub broker_identity: String,
    pub allowed_agents: Vec<AgentId>,
    pub allowed_operations: std::collections::BTreeSet<String>,
}
impl WriteWorkflow {
    pub fn load(path: &std::path::Path) -> Result<Self> {
        crate::security::root::verify_administrator_file(path)?;
        let c: Self = toml::from_str(&std::fs::read_to_string(path)?)?;
        c.validate()?;
        Ok(c)
    }
    fn validate(&self) -> Result<()> {
        let url = reqwest::Url::parse(&self.endpoint)?;
        if url.scheme() != "https"
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || self.name.is_empty()
            || self.credential.is_empty()
            || self.broker_identity.is_empty()
            || self.allowed_agents.is_empty()
            || self.allowed_operations.is_empty()
            || self
                .allowed_operations
                .iter()
                .any(|operation| !WriteManifest::supports_operation(operation))
        {
            bail!("Invalid reviewed write workflow")
        }
        Ok(())
    }
    fn check_manifest(&self, manifest: &WriteManifest) -> Result<()> {
        manifest.validate()?;
        if manifest
            .operations()
            .iter()
            .any(|operation| !self.allowed_operations.contains(*operation))
        {
            bail!("Manifest operations exceed this reviewed workflow's contract");
        }
        Ok(())
    }
    fn post(&self, credential: &str, body: &serde_json::Value) -> Result<serde_json::Value> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        let response = client
            .post(&self.endpoint)
            .bearer_auth(credential)
            .json(body)
            .send()
            .map_err(|_| anyhow::anyhow!("Gateway outcome unavailable"))?;
        if !response.status().is_success() {
            bail!("Gateway returned failed HTTP status")
        }
        let mut bytes = Vec::new();
        response.take(65_537).read_to_end(&mut bytes)?;
        if bytes.len() > 65_536 {
            bail!("Gateway response exceeds limit")
        }
        serde_json::from_slice(&bytes).map_err(|_| anyhow::anyhow!("Invalid gateway response"))
    }
}
#[derive(Serialize)]
pub struct WriteReview {
    pub id: String,
    pub workflow: String,
    pub task_id: String,
    pub manifest: WriteManifest,
    pub manifest_hash: String,
    pub expires_at: i64,
}
impl AgentHost {
    pub fn reconcile_write(
        &self,
        owner: AgentId,
        task: &str,
        session: &str,
        action: &str,
    ) -> Result<serde_json::Value> {
        self.task(owner, task)?;
        let store = WriteSessionStore::open(&self.root.join("writes.db"))?;
        let hash = store.uncertain_action_hash(action, session, owner, task)?;
        let workflow: String = self.write_reviews()?.query_row(
            "SELECT workflow FROM write_reviews WHERE id=?1 AND agent=?2 AND task=?3",
            params![session, owner.as_str(), task],
            |r| r.get(0),
        )?;
        let contract = Self::write_workflow(&workflow, owner)?;
        let request = ExecutionRequest::new(
            owner,
            task,
            "n8n.write",
            "n8n.write",
            &workflow,
            serde_json::json!({"phase":"status","action_id":action}),
        );
        self.execution_broker(&Self::execution_config()?)?
            .check_write(&request)?;
        let (gateway_session, _) = store.gateway_scope(session, owner, task)?;
        let result=SecretBroker::open(&self.root.join("credentials"),true)?.execute_redacted(owner,"n8n.write",&contract.credential,&request.id,|credential|contract.post(credential,&serde_json::json!({"phase":"status","session_id":gateway_session,"action_id":action,"agent":owner,"task_id":task})))?;
        let succeeded = reconciliation_succeeded(&result, action, &hash)?;
        self.audit(
            &request.id,
            owner,
            "write_reconciliation_evidence",
            &serde_json::json!({"action_id":action,"payload_hash":hash,"succeeded":succeeded})
                .to_string(),
        )?;
        store.reconcile(action, succeeded, &format!("audit:{}", request.id))?;
        Ok(
            serde_json::json!({"action_id":action,"status":if succeeded{"succeeded"}else{"reconciled_not_applied"}}),
        )
    }
    pub fn write_reviews_list(&self, owner: AgentId) -> Result<Vec<serde_json::Value>> {
        let db = self.write_reviews()?;
        let mut query=db.prepare("SELECT id,task,workflow,manifest,hash,expires,state FROM write_reviews WHERE agent=?1 ORDER BY rowid DESC LIMIT 100")?;
        let mut rows = Vec::new();
        for row in query.query_map([owner.as_str()], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, i64>(5)?,
                r.get::<_, String>(6)?,
            ))
        })? {
            let (id, task, workflow, manifest, hash, expires, state) = row?;
            rows.push(serde_json::json!({"id":id,"task_id":task,"workflow":workflow,"manifest":serde_json::from_str::<serde_json::Value>(&manifest)?,"manifest_hash":hash,"expires_at":expires,"state":state}));
        }
        Ok(rows)
    }
    fn write_workflow(name: &str, owner: AgentId) -> Result<WriteWorkflow> {
        let config = Self::execution_config()?;
        let contracts = config
            .write_contracts
            .iter()
            .map(|p| WriteWorkflow::load(p))
            .collect::<Result<Vec<_>>>()?;
        let c = contracts
            .into_iter()
            .find(|w| w.name == name && w.allowed_agents.contains(&owner))
            .ok_or_else(|| anyhow::anyhow!("Write workflow not granted"))?;
        Ok(c)
    }
    fn write_reviews(&self) -> Result<Connection> {
        let db = Connection::open(self.root.join("write-reviews.db"))?;
        db.busy_timeout(std::time::Duration::from_secs(5))?;
        db.execute_batch("CREATE TABLE IF NOT EXISTS write_reviews(id TEXT PRIMARY KEY,agent TEXT NOT NULL,task TEXT NOT NULL,workflow TEXT NOT NULL,manifest TEXT NOT NULL,hash TEXT NOT NULL,expires INTEGER NOT NULL,state TEXT NOT NULL);")?;
        Ok(db)
    }
    pub fn prepare_write(
        &self,
        owner: AgentId,
        task: &str,
        workflow: &str,
        manifest: WriteManifest,
    ) -> Result<WriteReview> {
        self.task(owner, task)?;
        Self::write_workflow(workflow, owner)?.check_manifest(&manifest)?;
        let review = WriteReview {
            id: uuid::Uuid::new_v4().to_string(),
            workflow: workflow.into(),
            task_id: task.into(),
            manifest_hash: manifest.hash()?,
            manifest,
            expires_at: chrono::Utc::now().timestamp() + 300,
        };
        self.write_reviews()?.execute(
            "INSERT INTO write_reviews VALUES(?1,?2,?3,?4,?5,?6,?7,'pending')",
            params![
                review.id,
                owner.as_str(),
                task,
                workflow,
                serde_json::to_string(&review.manifest)?,
                review.manifest_hash,
                review.expires_at
            ],
        )?;
        self.audit(
            &review.id,
            owner,
            "write_review_prepared",
            &review.manifest_hash,
        )?;
        Ok(review)
    }
    /// Only the dedicated authenticated human endpoint calls this. Do not log inputs/errors.
    pub fn authorize_write(
        &self,
        owner: AgentId,
        id: &str,
        reviewed_hash: &str,
        code: &str,
    ) -> Result<serde_json::Value> {
        if code.len() != 6 || !code.bytes().all(|b| b.is_ascii_digit()) {
            bail!("Invalid authorization code")
        }
        let mut db = self.write_reviews()?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (task,workflow,manifest):(String,String,String)=tx.query_row("SELECT task,workflow,manifest FROM write_reviews WHERE id=?1 AND agent=?2 AND hash=?3 AND expires>?4 AND state='pending'",params![id,owner.as_str(),reviewed_hash,chrono::Utc::now().timestamp()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;
        self.task(owner, &task)?;
        tx.execute(
            "UPDATE write_reviews SET state='authorizing' WHERE id=?1",
            [id],
        )?;
        tx.commit()?;
        let result =
            (|| -> Result<serde_json::Value> {
                let manifest: WriteManifest = serde_json::from_str(&manifest)?;
                let contract = Self::write_workflow(&workflow, owner)?;
                let mut token_broker = SecretBroker::open(&self.root.join("credentials"), true)?;
                let secrets = SecretBroker::open(&self.root.join("credentials"), true)?;
                let result =
            secrets.execute_redacted(owner, "n8n.write", &contract.credential, id, |credential| {
                let grant = authorize_gateway(
                    &contract,
                    owner,
                    id,
                    &task,
                    &manifest,
                    reviewed_hash,
                    code,
                    |body| contract.post(credential, body),
                )?;
                validate_authorization_metadata(&grant, credential, code)?;
                let gateway_request = &grant.request_id;
                let session = &grant.session_id;
                let token = &grant.token;
                let expires = grant.expires_at;
                let audit = &grant.audit_id;
                let reference = format!("write-{id}");
                token_broker.register_expiring(
                    &reference,
                    vec![owner],
                    vec!["n8n.write.session".into()],
                    token,
                    expires,
                )?;
                WriteSessionStore::open(&self.root.join("writes.db"))?.record_verified(
                    &VerifiedWriteGrant {
                        id: id.into(),
                        agent: owner,
                        task_id: task.clone(),
                        broker_identity: contract.broker_identity.clone(),
                        manifest: manifest.clone(),
                        reviewed_hash: reviewed_hash.into(),
                        expires_at: expires,
                        gateway_request_id: gateway_request.clone(),
                        gateway_session_id: session.clone(),
                        gateway_audit_id: audit.clone(),
                        credential_reference: reference,
                    },
                )?;
                Ok(serde_json::json!({"session_id":id,"expires_at":expires,"authorized":true}))
            });
                if result.is_err() {
                    token_broker.revoke_reference(&format!("write-{id}"))?;
                }
                result
            })();
        self.write_reviews()?.execute(
            "UPDATE write_reviews SET state=?1 WHERE id=?2",
            params![
                if result.is_ok() {
                    "authorized"
                } else {
                    "failed"
                },
                id
            ],
        )?;
        self.audit(
            id,
            owner,
            if result.is_ok() {
                "write_authorized"
            } else {
                "write_authorization_failed"
            },
            reviewed_hash,
        )?;
        result
    }
    #[allow(clippy::too_many_arguments)]
    pub fn dispatch_write(
        &self,
        owner: AgentId,
        task: &str,
        session: &str,
        operation: &str,
        payload: &serde_json::Value,
        request_id: Option<&str>,
    ) -> Result<serde_json::Value> {
        self.task(owner, task)?;
        let db = self.write_reviews()?;
        let workflow:String=db.query_row("SELECT workflow FROM write_reviews WHERE id=?1 AND agent=?2 AND task=?3 AND state='authorized'",params![session,owner.as_str(),task],|r|r.get(0))?;
        let contract = Self::write_workflow(&workflow, owner)?;
        if !contract.allowed_operations.contains(operation) {
            bail!("Operation is outside the current reviewed workflow contract");
        }
        let mut request = ExecutionRequest::new(
            owner,
            task,
            "n8n.write",
            "n8n.write",
            &workflow,
            serde_json::json!({"operation":operation,"payload":payload,"session_id":session}),
        );
        if let Some(id) = request_id {
            request.id = uuid::Uuid::parse_str(id)?.to_string();
        }
        self.execution_broker(&Self::execution_config()?)?
            .check_write(&request)?;
        let store = WriteSessionStore::open(&self.root.join("writes.db"))?;
        store.reserve(
            session,
            &request.id,
            owner,
            task,
            &contract.broker_identity,
            operation,
            payload,
        )?;
        let secrets = SecretBroker::open(&self.root.join("credentials"), true)?;
        let (gateway_session, reference) = store.gateway_scope(session, owner, task)?;
        let result = secrets.execute_redacted(owner, "n8n.write", &contract.credential, &request.id, |_| {
            secrets.execute_redacted(owner, "n8n.write.session", &reference, &request.id, |token| {
                contract.post(token, &serde_json::json!({"phase":"action","session_id":gateway_session,"action_id":request.id,"agent":owner,"task_id":task,"op":operation,"payload":payload}))
            })
        });
        record_dispatch_outcome(&store, &request.id, &result)
    }
    pub fn write_status(
        &self,
        owner: AgentId,
        task: &str,
        session: &str,
    ) -> Result<serde_json::Value> {
        self.task(owner, task)?;
        Ok(serde_json::to_value(
            WriteSessionStore::open(&self.root.join("writes.db"))?.status(session, owner, task)?,
        )?)
    }
    pub fn end_write(
        &self,
        owner: AgentId,
        task: &str,
        session: &str,
    ) -> Result<serde_json::Value> {
        self.task(owner, task)?;
        let store = WriteSessionStore::open(&self.root.join("writes.db"))?;
        close_write_session(
            &store,
            &self.root.join("credentials"),
            session,
            owner,
            task,
            |token, gateway_session| {
                let workflow: String = self.write_reviews()?.query_row(
                    "SELECT workflow FROM write_reviews WHERE id=?1 AND agent=?2 AND task=?3",
                    params![session, owner.as_str(), task],
                    |r| r.get(0),
                )?;
                let c = Self::write_workflow(&workflow, owner)?;
                c.post(
                    token,
                    &serde_json::json!({"phase":"end","session_id":gateway_session}),
                )
            },
        )
    }
}

fn record_dispatch_outcome(
    store: &WriteSessionStore,
    action: &str,
    result: &Result<serde_json::Value>,
) -> Result<serde_json::Value> {
    let response = result
        .as_ref()
        .ok()
        .filter(|value| value.get("action_id").and_then(serde_json::Value::as_str) == Some(action));
    let correlation = |field| {
        response
            .and_then(|value| value.get(field))
            .and_then(serde_json::Value::as_str)
            .filter(|id| crate::n8n::valid_correlation_id(id))
    };
    let audit = correlation("audit_id");
    let execution = correlation("execution_id");
    let succeeded =
        response.is_some_and(gateway_succeeded) && audit.is_some() && execution.is_some();
    store.record_outcome(
        action,
        if succeeded {
            WriteOutcome::Succeeded
        } else {
            WriteOutcome::Uncertain
        },
        response.map(|_| action),
        audit,
        execution,
    )?;
    Ok(
        serde_json::json!({"action_id":action,"status":if succeeded{"succeeded"}else{"uncertain"},"retry_allowed":false}),
    )
}

fn reconciliation_succeeded(result: &serde_json::Value, action: &str, hash: &str) -> Result<bool> {
    if !gateway_succeeded(result)
        || result.get("action_id").and_then(serde_json::Value::as_str) != Some(action)
        || result
            .get("payload_hash")
            .and_then(serde_json::Value::as_str)
            != Some(hash)
        || result.get("final").and_then(serde_json::Value::as_bool) != Some(true)
    {
        bail!("Gateway evidence is not final or does not bind this action");
    }
    match result.get("status").and_then(serde_json::Value::as_str) {
        Some("succeeded") => Ok(true),
        Some("not_applied") => Ok(false),
        _ => bail!("Gateway cannot resolve this uncertain action"),
    }
}

fn gateway_succeeded(value: &serde_json::Value) -> bool {
    value.get("ok").and_then(serde_json::Value::as_bool) == Some(true)
        && value.get("error").is_none_or(serde_json::Value::is_null)
}

fn close_write_session(
    store: &WriteSessionStore,
    credential_root: &std::path::Path,
    session: &str,
    owner: AgentId,
    task: &str,
    close: impl FnOnce(&str, &str) -> Result<serde_json::Value>,
) -> Result<serde_json::Value> {
    store.end(session, owner, task)?;
    if store.status(session, owner, task)?.gateway_end_confirmed == Some(true) {
        return Ok(serde_json::json!({"revoked":true,"gateway_end_confirmed":true}));
    }
    let (gateway_session, reference) = store.gateway_scope(session, owner, task)?;
    let mut secrets = SecretBroker::open(credential_root, true)?;
    let id = uuid::Uuid::new_v4().to_string();
    let ended = secrets
        .execute_redacted(owner, "n8n.write.session", &reference, &id, |token| {
            close(token, &gateway_session)
        })
        .is_ok_and(|value| {
            gateway_succeeded(&value)
                && value
                    .get("session_id")
                    .is_none_or(|id| id.as_str() == Some(gateway_session.as_str()))
        });
    secrets.revoke_reference(&reference)?;
    store.record_closure_status(session, owner, task, ended)?;
    Ok(serde_json::json!({"revoked":true,"gateway_end_confirmed":ended}))
}

// Deliberately no Debug/Serialize: bearer material never becomes a result payload.
struct GatewayAuthorization {
    request_id: String,
    session_id: String,
    token: String,
    expires_at: i64,
    audit_id: String,
}
fn validate_authorization_metadata(
    grant: &GatewayAuthorization,
    credential: &str,
    code: &str,
) -> Result<()> {
    let metadata = serde_json::json!({"request_id":grant.request_id,"session_id":grant.session_id,"audit_id":grant.audit_id});
    let mut redacted = metadata.clone();
    for secret in [credential, grant.token.as_str(), code] {
        crate::security::broker::redact(&mut redacted, secret);
    }
    if redacted != metadata {
        bail!("Gateway authorization metadata contains sensitive material");
    }
    Ok(())
}
#[allow(clippy::too_many_arguments)]
fn authorize_gateway(
    contract: &WriteWorkflow,
    owner: AgentId,
    id: &str,
    task: &str,
    manifest: &WriteManifest,
    hash: &str,
    code: &str,
    mut post: impl FnMut(&serde_json::Value) -> Result<serde_json::Value>,
) -> Result<GatewayAuthorization> {
    contract.check_manifest(manifest)?;
    if !contract.allowed_agents.contains(&owner) || manifest.hash()? != hash {
        bail!("Authorization request does not match the reviewed owner and manifest");
    }
    let requested = post(
        &serde_json::json!({"phase":"request","request_id":id,"agent":owner,"task_id":task,"broker_identity":contract.broker_identity,"manifest":manifest,"manifest_hash":hash}),
    )?;
    let field = |value: &serde_json::Value, key: &str| -> Result<String> {
        let text = value
            .get(key)
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("Gateway required field missing"))?;
        if text.is_empty() || text.len() > 16_384 || text.chars().any(char::is_control) {
            bail!("Invalid gateway field")
        }
        if key != "session_token" && !crate::n8n::valid_correlation_id(text) {
            bail!("Invalid gateway correlation identifier");
        }
        Ok(text.into())
    };
    let request_id = field(&requested, "request_id")?;
    if !gateway_succeeded(&requested)
        || requested
            .get("manifest_hash")
            .and_then(serde_json::Value::as_str)
            != Some(hash)
    {
        bail!("Gateway reviewed manifest mismatch")
    }
    let authorized = post(
        &serde_json::json!({"phase":"authorize","request_id":request_id,"manifest_hash":hash,"totp":code}),
    )?;
    if !gateway_succeeded(&authorized)
        || authorized
            .get("manifest_hash")
            .and_then(serde_json::Value::as_str)
            != Some(hash)
        || authorized.get("agent").and_then(serde_json::Value::as_str) != Some(owner.as_str())
        || authorized
            .get("task_id")
            .and_then(serde_json::Value::as_str)
            != Some(task)
        || authorized
            .get("broker_identity")
            .and_then(serde_json::Value::as_str)
            != Some(contract.broker_identity.as_str())
    {
        bail!("Gateway authorization scope mismatch")
    }
    let expires_at = authorized
        .get("expires_at")
        .and_then(serde_json::Value::as_i64)
        .ok_or_else(|| anyhow::anyhow!("Gateway expiry missing"))?;
    let now = chrono::Utc::now().timestamp();
    if expires_at <= now || expires_at > now + 300 {
        bail!("Invalid gateway session expiry")
    }
    Ok(GatewayAuthorization {
        request_id,
        session_id: field(&authorized, "session_id")?,
        token: field(&authorized, "session_token")?,
        expires_at,
        audit_id: field(&authorized, "audit_id")?,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use sha2::Digest;
    #[test]
    fn both_dispatch_credentials_are_redacted_before_correlation_is_persisted() {
        let directory = tempfile::TempDir::new().unwrap();
        let grant = grant();
        let (store, mut secrets) = storage(directory.path(), &grant);
        secrets
            .register(
                "webhook",
                vec![grant.agent],
                vec!["n8n.write".into()],
                "fixture-webhook",
            )
            .unwrap();
        let action = uuid::Uuid::new_v4().to_string();
        store
            .reserve(
                &grant.id,
                &action,
                grant.agent,
                &grant.task_id,
                &grant.broker_identity,
                "email.send",
                &payload(&grant),
            )
            .unwrap();
        let result = secrets.execute_redacted(grant.agent, "n8n.write", "webhook", &action, |_| {
            secrets.execute_redacted(grant.agent, "n8n.write.session", &grant.credential_reference, &action, |token| {
                Ok(serde_json::json!({"ok":true,"action_id":action,"audit_id":"fixture-webhook","execution_id":"execution","echo":token}))
            })
        });
        assert_eq!(
            record_dispatch_outcome(&store, &action, &result).unwrap()["status"],
            "uncertain"
        );
        assert!(store
            .status(&grant.id, grant.agent, &grant.task_id)
            .unwrap()
            .actions[0]
            .gateway_audit_id
            .is_none());
        let bytes = std::fs::read(directory.path().join("writes.db")).unwrap();
        for secret in ["fixture-webhook", "fixture-session-token"] {
            assert!(!bytes
                .windows(secret.len())
                .any(|window| window == secret.as_bytes()));
        }
    }
    #[test]
    fn sensitive_authorization_metadata_is_rejected_before_persistence() {
        let mut authorization = GatewayAuthorization {
            request_id: "request".into(),
            session_id: "session".into(),
            token: "fixture-session-token".into(),
            expires_at: chrono::Utc::now().timestamp() + 60,
            audit_id: "audit".into(),
        };
        validate_authorization_metadata(&authorization, "fixture-webhook", "918273").unwrap();
        for secret in ["fixture-session-token", "fixture-webhook", "918273"] {
            authorization.audit_id = secret.into();
            assert!(
                validate_authorization_metadata(&authorization, "fixture-webhook", "918273")
                    .is_err()
            );
            authorization.audit_id = hex::encode_upper(secret.as_bytes());
            assert!(
                validate_authorization_metadata(&authorization, "fixture-webhook", "918273")
                    .is_err()
            );
        }
    }
    #[test]
    fn reviewed_workflow_operations_and_manifest_hash_are_required_before_transport() {
        let smtp: WriteWorkflow =
            toml::from_str(include_str!("../../config/n8n-write.example.toml")).unwrap();
        let mailbox: WriteWorkflow =
            toml::from_str(include_str!("../../config/n8n-mailbox-write.example.toml")).unwrap();
        smtp.validate().unwrap();
        mailbox.validate().unwrap();
        let grant = grant();
        assert!(smtp.check_manifest(&grant.manifest).is_ok());
        assert!(mailbox.check_manifest(&grant.manifest).is_err());
        assert!(authorize_gateway(
            &smtp,
            AgentId::Janet,
            &grant.id,
            &grant.task_id,
            &grant.manifest,
            "changed-hash",
            "918273",
            |_| panic!("Unreviewed manifest reached transport")
        )
        .is_err());
        assert!(authorize_gateway(
            &smtp,
            AgentId::Harley,
            &grant.id,
            &grant.task_id,
            &grant.manifest,
            &grant.reviewed_hash,
            "918273",
            |_| panic!("Peer scope reached transport")
        )
        .is_err());
    }
    #[test]
    fn failed_authorization_is_burned_and_leaves_a_terminal_review_state() {
        let directory = tempfile::TempDir::new().unwrap();
        let host = AgentHost::open(directory.path()).unwrap();
        let task = host
            .create_task(AgentId::Janet, "Fixture", "Internal fixture only", None)
            .unwrap();
        let grant = grant();
        let database = host.write_reviews().unwrap();
        database
            .execute(
                "INSERT INTO write_reviews VALUES(?1,'janet',?2,?3,?4,?5,?6,'pending')",
                params![
                    grant.id,
                    task.id,
                    format!("fixture-unregistered-{}", uuid::Uuid::new_v4()),
                    serde_json::to_string(&grant.manifest).unwrap(),
                    grant.reviewed_hash,
                    grant.expires_at
                ],
            )
            .unwrap();
        for _ in 0..2 {
            assert!(host
                .authorize_write(AgentId::Janet, &grant.id, &grant.reviewed_hash, "918273")
                .is_err());
            let state: String = database
                .query_row(
                    "SELECT state FROM write_reviews WHERE id=?1",
                    [&grant.id],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(state, "failed");
        }
    }
    fn grant() -> VerifiedWriteGrant {
        let manifest = WriteManifest::SingleAction {
            operation: "email.send".into(),
            payload: serde_json::json!({"to":["fixture@example.invalid"],"body":"reviewed fixture"}),
        };
        let id = uuid::Uuid::new_v4().to_string();
        VerifiedWriteGrant {
            credential_reference: format!("write-{id}"),
            id,
            agent: AgentId::Janet,
            task_id: "task".into(),
            broker_identity: "broker".into(),
            reviewed_hash: manifest.hash().unwrap(),
            manifest,
            expires_at: chrono::Utc::now().timestamp() + 60,
            gateway_request_id: "gateway-request".into(),
            gateway_session_id: "gateway-session".into(),
            gateway_audit_id: "authorization-audit".into(),
        }
    }
    fn storage(
        root: &std::path::Path,
        grant: &VerifiedWriteGrant,
    ) -> (WriteSessionStore, SecretBroker) {
        let credentials = root.join("credentials");
        super::super::private_directory(&credentials).unwrap();
        let mut secrets = SecretBroker::open(&credentials, true).unwrap();
        secrets
            .register_expiring(
                &grant.credential_reference,
                vec![grant.agent],
                vec!["n8n.write.session".into()],
                "fixture-session-token",
                grant.expires_at,
            )
            .unwrap();
        let store = WriteSessionStore::open(&root.join("writes.db")).unwrap();
        store.record_verified(grant).unwrap();
        (store, secrets)
    }
    fn payload(grant: &VerifiedWriteGrant) -> serde_json::Value {
        let WriteManifest::SingleAction { payload, .. } = &grant.manifest else {
            unreachable!()
        };
        payload.clone()
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn local_gateway_authorization_dispatch_restart_and_close_are_correlated() {
        struct SafeGuard;
        impl crate::security::Guard for SafeGuard {
            fn assess(&self, _: &ExecutionRequest) -> crate::security::GuardDecision {
                crate::security::GuardDecision::Safe
            }
        }
        let directory = tempfile::TempDir::new().unwrap();
        let grant = grant();
        let action = uuid::Uuid::new_v4().to_string();
        let (endpoint, server) = crate::n8n::test_gateway::start(vec![
            serde_json::json!({"ok":true,"request_id":grant.gateway_request_id,"manifest_hash":grant.reviewed_hash}),
            serde_json::json!({"ok":true,"agent":"janet","task_id":"task","broker_identity":"broker","manifest_hash":grant.reviewed_hash,"session_id":grant.gateway_session_id,"session_token":"fixture-session-token","expires_at":grant.expires_at,"audit_id":grant.gateway_audit_id}),
            serde_json::json!({"ok":true,"action_id":action,"audit_id":"action-audit","execution_id":"execution-1","session_token":"untrusted-token-echo"}),
            serde_json::json!({"ok":true,"session_id":grant.gateway_session_id}),
        ]);
        let contract = WriteWorkflow {
            name: "mail".into(),
            endpoint,
            credential: "webhook".into(),
            broker_identity: "broker".into(),
            allowed_agents: vec![AgentId::Janet],
            allowed_operations: std::collections::BTreeSet::from(["email.send".into()]),
        };
        let authorized = authorize_gateway(
            &contract,
            grant.agent,
            &grant.id,
            &grant.task_id,
            &grant.manifest,
            &grant.reviewed_hash,
            "918273",
            |body| contract.post("fixture-webhook", body),
        )
        .unwrap();
        assert_eq!(authorized.session_id, grant.gateway_session_id);
        assert_eq!(authorized.request_id, grant.gateway_request_id);
        assert_eq!(authorized.audit_id, grant.gateway_audit_id);
        assert_eq!(authorized.token, "fixture-session-token");
        let (store, secrets) = storage(directory.path(), &grant);
        let request = ExecutionRequest {
            id: action.clone(),
            ..ExecutionRequest::new(
                grant.agent,
                &grant.task_id,
                "n8n.write",
                "n8n.write",
                "mail",
                payload(&grant),
            )
        };
        let broker = crate::security::ExecutionBroker::with_policy(
            crate::security::ApprovalStore::open(&directory.path().join("approvals.db")).unwrap(),
            SafeGuard,
            crate::security::root::RootPolicy::fixture(
                grant.agent,
                "n8n.write",
                "n8n.write",
                "mail",
            ),
        );
        broker.check_write(&request).unwrap();
        store
            .reserve(
                &grant.id,
                &action,
                grant.agent,
                &grant.task_id,
                &grant.broker_identity,
                "email.send",
                &payload(&grant),
            )
            .unwrap();
        let reply = secrets.execute_redacted(grant.agent, "n8n.write.session", &grant.credential_reference, &action, |token| contract.post(token, &serde_json::json!({"phase":"action","session_id":grant.gateway_session_id,"action_id":action,"payload":payload(&grant)})));
        let result = record_dispatch_outcome(&store, &action, &reply).unwrap();
        assert_eq!(result["status"], "succeeded");
        assert!(!result.to_string().contains("token"));
        drop(store);
        let reopened = WriteSessionStore::open(&directory.path().join("writes.db")).unwrap();
        let status = reopened
            .status(&grant.id, grant.agent, &grant.task_id)
            .unwrap();
        assert_eq!(
            status.actions[0].gateway_action_id.as_deref(),
            Some(action.as_str())
        );
        assert_eq!(
            status.actions[0].gateway_audit_id.as_deref(),
            Some("action-audit")
        );
        assert_eq!(
            status.actions[0].n8n_execution_id.as_deref(),
            Some("execution-1")
        );
        assert_eq!(status.remaining["email.send"], 0);
        let ended = close_write_session(
            &reopened,
            &directory.path().join("credentials"),
            &grant.id,
            grant.agent,
            &grant.task_id,
            |token, session| {
                assert!(
                    reopened
                        .status(&grant.id, grant.agent, &grant.task_id)
                        .unwrap()
                        .revoked
                );
                contract.post(
                    token,
                    &serde_json::json!({"phase":"end","session_id":session}),
                )
            },
        )
        .unwrap();
        assert_eq!(ended["gateway_end_confirmed"], true);
        let repeated = close_write_session(
            &reopened,
            &directory.path().join("credentials"),
            &grant.id,
            grant.agent,
            &grant.task_id,
            |_, _| panic!("Confirmed closure was repeated remotely"),
        )
        .unwrap();
        assert_eq!(repeated["gateway_end_confirmed"], true);
        assert_eq!(
            reopened
                .status(&grant.id, grant.agent, &grant.task_id)
                .unwrap()
                .gateway_end_confirmed,
            Some(true)
        );
        assert!(secrets
            .references_for(grant.agent, "n8n.write.session")
            .unwrap()
            .is_empty());
        let captured = server.join().unwrap();
        assert_eq!(captured.len(), 4);
        let wire_hash = hex::encode(sha2::Sha256::digest(
            serde_json::to_vec(&captured[0].1["manifest"]).unwrap(),
        ));
        assert_eq!(captured[0].1["manifest_hash"], wire_hash);
        assert_eq!(captured[0].0, "Bearer fixture-webhook");
        assert_eq!(captured[2].0, "Bearer fixture-session-token");
        assert_eq!(captured[1].1["totp"], "918273");
        for index in [0, 2, 3] {
            assert!(captured[index].1.get("totp").is_none());
        }
    }

    #[test]
    fn incomplete_dispatch_evidence_stays_uncertain_and_closure_always_revokes() {
        for response in [
            serde_json::json!({"ok":true}),
            serde_json::json!({"ok":false,"retry_allowed":true}),
            serde_json::json!({"ok":true,"audit_id":"x".repeat(129),"execution_id":"execution"}),
        ] {
            let directory = tempfile::TempDir::new().unwrap();
            let grant = grant();
            let (store, secrets) = storage(directory.path(), &grant);
            let action = uuid::Uuid::new_v4().to_string();
            store
                .reserve(
                    &grant.id,
                    &action,
                    grant.agent,
                    &grant.task_id,
                    &grant.broker_identity,
                    "email.send",
                    &payload(&grant),
                )
                .unwrap();
            let mut response = response;
            response["action_id"] = serde_json::json!(action);
            assert_eq!(
                record_dispatch_outcome(&store, &action, &Ok(response)).unwrap()["status"],
                "uncertain"
            );
            assert!(store
                .uncertain_action_hash(&action, &grant.id, grant.agent, &grant.task_id)
                .is_ok());
            let ended = close_write_session(
                &store,
                &directory.path().join("credentials"),
                &grant.id,
                grant.agent,
                &grant.task_id,
                |_, _| Ok(serde_json::json!({"ok":false})),
            )
            .unwrap();
            assert_eq!(ended["revoked"], true);
            assert_eq!(ended["gateway_end_confirmed"], false);
            assert_eq!(
                WriteSessionStore::open(&directory.path().join("writes.db"))
                    .unwrap()
                    .status(&grant.id, grant.agent, &grant.task_id)
                    .unwrap()
                    .gateway_end_confirmed,
                Some(false)
            );
            assert!(secrets
                .references_for(grant.agent, "n8n.write.session")
                .unwrap()
                .is_empty());
        }
        let directory = tempfile::TempDir::new().unwrap();
        let grant = grant();
        let (store, secrets) = storage(directory.path(), &grant);
        let ended = close_write_session(
            &store,
            &directory.path().join("credentials"),
            &grant.id,
            grant.agent,
            &grant.task_id,
            |_, _| anyhow::bail!("fixture connection failure"),
        )
        .unwrap();
        assert_eq!(ended["gateway_end_confirmed"], false);
        assert!(secrets
            .references_for(grant.agent, "n8n.write.session")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn retry_flag_never_substitutes_for_final_exact_reconciliation_evidence() {
        let valid = serde_json::json!({"ok":true,"final":true,"action_id":"action","payload_hash":"hash","status":"not_applied"});
        assert!(!reconciliation_succeeded(&valid, "action", "hash").unwrap());
        for (field, value) in [
            ("ok", serde_json::json!(false)),
            ("final", serde_json::json!(false)),
            ("action_id", serde_json::json!("other")),
            ("payload_hash", serde_json::json!("other")),
            ("status", serde_json::json!("unknown")),
        ] {
            let mut invalid = valid.clone();
            invalid[field] = value;
            invalid["retry_allowed"] = serde_json::json!(true);
            assert!(reconciliation_succeeded(&invalid, "action", "hash").is_err());
        }
    }
    #[test]
    fn authorization_binds_manifest_owner_task_broker_and_expiry() {
        let contract = WriteWorkflow {
            name: "mail".into(),
            endpoint: "https://example.invalid/gateway".into(),
            credential: "mail_credential".into(),
            broker_identity: "vm".into(),
            allowed_agents: vec![AgentId::Janet],
            allowed_operations: std::collections::BTreeSet::from(["email.send".into()]),
        };
        let manifest = WriteManifest::SingleAction {
            operation: "email.send".into(),
            payload: serde_json::json!({"to":["test@example.invalid"],"body":"reviewed"}),
        };
        let hash = manifest.hash().unwrap();
        let authorized = serde_json::json!({"ok":true,"manifest_hash":hash,"agent":"janet","task_id":"task","broker_identity":"vm","session_id":"gateway-session","session_token":"fake-bearer","audit_id":"audit","expires_at":chrono::Utc::now().timestamp()+60});
        let run = |response: serde_json::Value| {
            let mut calls = 0;
            authorize_gateway(
                &contract,
                AgentId::Janet,
                "review",
                "task",
                &manifest,
                &hash,
                "918273",
                |body| {
                    calls += 1;
                    if calls == 1 {
                        assert!(body.get("totp").is_none());
                        Ok(
                            serde_json::json!({"ok":true,"request_id":"gateway-request","manifest_hash":hash}),
                        )
                    } else {
                        assert_eq!(body["totp"], "918273");
                        Ok(response.clone())
                    }
                },
            )
        };
        let grant = run(authorized.clone()).unwrap();
        assert_eq!(grant.session_id, "gateway-session");
        assert_eq!(grant.token, "fake-bearer");
        for (key, value) in [
            ("manifest_hash", serde_json::json!("changed")),
            ("agent", serde_json::json!("harley")),
            ("task_id", serde_json::json!("other")),
            ("broker_identity", serde_json::json!("other")),
            ("expires_at", serde_json::json!(0)),
            ("ok", serde_json::json!(false)),
        ] {
            let mut altered = authorized.clone();
            altered[key] = value;
            assert!(run(altered).is_err());
        }
    }
}
