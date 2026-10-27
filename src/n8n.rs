//! Reviewed read-only workflow adapters. No arbitrary URL or write invocation.
//! Infrastructure constructs the registry and credentials; models supply named
//! references and bounded payloads. Live wiring remains behind deployment gates.
use crate::assistants::{AgentHost, AgentId};
use crate::security::{Admission, ExecutionBroker, ExecutionRequest, Guard, SecretBroker};
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadWorkflow {
    name: String,
    endpoint: String,
    credential: String,
    allowed_agents: BTreeSet<String>,
    allowed_operations: BTreeSet<String>,
    fields: BTreeMap<String, FieldRule>,
    required_fields: BTreeMap<String, BTreeSet<String>>,
    response_fields: BTreeSet<String>,
    #[serde(default)]
    response_format: ReadResponseFormat,
}

#[derive(Default, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum ReadResponseFormat {
    #[default]
    ExplicitOk,
    EmailReadonlyV2,
}
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum FieldRule {
    Text { max_bytes: usize },
    Integer { min: u64, max: u64 },
    Choice { values: BTreeSet<String> },
}
impl ReadWorkflow {
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Only host configuration may register reviewed read contracts. An arbitrary
    /// workflow marked "read" by a model is never accepted.
    pub fn load(path: &std::path::Path) -> Result<Self> {
        crate::security::root::verify_administrator_file(path)?;
        let workflow: Self = toml::from_str(&std::fs::read_to_string(path)?)?;
        workflow.validate()?;
        Ok(workflow)
    }
    fn validate(&self) -> Result<()> {
        let endpoint = reqwest::Url::parse(&self.endpoint)?;
        if self.name.is_empty()
            || self.credential.is_empty()
            || endpoint.scheme() != "https"
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.query().is_some()
            || endpoint.fragment().is_some()
            || self.allowed_agents.is_empty()
            || self
                .allowed_agents
                .iter()
                .any(|id| AgentId::parse(id).is_err())
            || self.allowed_operations.is_empty()
            || self.fields.len() > 20
            || self.response_fields.is_empty()
            || self.response_fields.len() > 20
            || self.response_fields.iter().any(|name| {
                [
                    "token",
                    "secret",
                    "password",
                    "authorization",
                    "session_token",
                    "totp",
                ]
                .contains(&name.to_ascii_lowercase().as_str())
            })
            || self
                .allowed_operations
                .iter()
                .any(|op| op.trim().is_empty())
        {
            bail!("Invalid administrator-reviewed read workflow");
        }
        for field in self.fields.values() {
            match field {
                FieldRule::Text { max_bytes } if *max_bytes == 0 || *max_bytes > 2_000 => {
                    bail!("Invalid workflow field limit")
                }
                FieldRule::Integer { min, max } if min > max => {
                    bail!("Invalid workflow integer bounds")
                }
                FieldRule::Choice { values } if values.is_empty() => {
                    bail!("Workflow choice allowlist cannot be empty")
                }
                _ => {}
            }
        }
        if self
            .required_fields
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>()
            != self.allowed_operations
            || self
                .required_fields
                .values()
                .any(|fields| fields.iter().any(|name| !self.fields.contains_key(name)))
        {
            bail!("Every read operation requires an explicit required-field contract");
        }
        Ok(())
    }
    fn response_succeeded(&self, result: &serde_json::Value, payload: &serde_json::Value) -> bool {
        if self.response_format == ReadResponseFormat::ExplicitOk {
            return result.get("ok").and_then(serde_json::Value::as_bool) == Some(true)
                && result.get("error").is_none_or(serde_json::Value::is_null);
        }
        let op = payload.get("op").and_then(serde_json::Value::as_str);
        let shape = match op {
            Some("read") => result.get("body").is_some_and(serde_json::Value::is_string),
            Some("search" | "recent" | "unread") => result
                .get("messages")
                .is_some_and(serde_json::Value::is_array),
            Some("attachment_list") => result
                .get("attachments")
                .is_some_and(serde_json::Value::is_array),
            _ => false,
        };
        shape
            && result.get("read_only").and_then(serde_json::Value::as_bool) == Some(true)
            && result.get("op") == payload.get("op")
            && result.get("mailbox") == payload.get("mailbox")
            && (payload.get("uid").is_none() || result.get("uid") == payload.get("uid"))
            && result.get("error").is_none()
            && result.get("code").is_none()
            && result.get("ok").is_none_or(|v| v.as_bool() == Some(true))
    }

    fn check_input(&self, owner: AgentId, payload: &serde_json::Value) -> Result<()> {
        if !self.allowed_agents.contains(owner.as_str()) {
            bail!("Workflow is not granted to this agent");
        }
        let object = payload
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("Workflow payload must be an object"))?;
        let operation = object
            .get("op")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        if !self.allowed_operations.contains(operation)
            || serde_json::to_vec(payload)?.len() > 8_000
        {
            bail!("Workflow operation is not reviewed or payload exceeds limit");
        }
        if self
            .required_fields
            .get(operation)
            .is_none_or(|fields| fields.iter().any(|name| !object.contains_key(name)))
        {
            bail!("Required read workflow input is missing");
        }
        for (name, value) in object {
            if name == "op" {
                continue;
            }
            let field = self
                .fields
                .get(name)
                .ok_or_else(|| anyhow::anyhow!("Unknown workflow input field"))?;
            let valid = match field {
                FieldRule::Text { max_bytes } => value.as_str().is_some_and(|text| {
                    text.len() <= *max_bytes && !text.chars().any(char::is_control)
                }),
                FieldRule::Integer { min, max } => value
                    .as_u64()
                    .is_some_and(|number| number >= *min && number <= *max),
                FieldRule::Choice { values } => {
                    value.as_str().is_some_and(|text| values.contains(text))
                }
            };
            if !valid {
                bail!("Workflow input is outside the reviewed contract");
            }
        }
        Ok(())
    }
}
#[derive(Debug, Serialize)]
pub struct ReadResult {
    pub request_id: String,
    pub agent: AgentId,
    pub task_id: String,
    pub workflow: String,
    pub gateway_audit_id: Option<String>,
    pub n8n_execution_id: Option<String>,
    pub result: serde_json::Value,
}
/// Trusted transport abstraction enables failure/isolation tests without live n8n.
pub trait ReadTransport {
    fn post(
        &self,
        endpoint: &str,
        credential: &str,
        request: &ExecutionRequest,
    ) -> Result<serde_json::Value>;
}
pub struct HttpsReadTransport {
    client: reqwest::blocking::Client,
}
impl HttpsReadTransport {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(20))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
        })
    }
}
impl ReadTransport for HttpsReadTransport {
    fn post(
        &self,
        endpoint: &str,
        credential: &str,
        request: &ExecutionRequest,
    ) -> Result<serde_json::Value> {
        let response = self
            .client
            .post(endpoint)
            .bearer_auth(credential)
            .header("X-Request-ID", &request.id)
            .header("X-Agent-ID", request.agent.as_str())
            .header("X-Task-ID", &request.task_id)
            .json(&request.payload)
            .send()
            .map_err(|_| anyhow::anyhow!("n8n read request failed"))?;
        if !response.status().is_success() {
            bail!("n8n read workflow returned a failed HTTP status");
        }
        let execution_id = response
            .headers()
            .get("X-N8n-Execution-ID")
            .map(|value| value.to_str().map(str::to_owned))
            .transpose()
            .map_err(|_| anyhow::anyhow!("Invalid n8n execution correlation header"))?;
        let audit_id = response
            .headers()
            .get("X-N8n-Audit-ID")
            .map(|value| value.to_str().map(str::to_owned))
            .transpose()
            .map_err(|_| anyhow::anyhow!("Invalid n8n audit correlation header"))?;
        let mut bytes = Vec::new();
        response.take(1_048_577).read_to_end(&mut bytes)?;
        if bytes.len() > 1_048_576 {
            bail!("n8n response exceeds the 1 MiB read limit; use bounded artifact ingestion for large attachments");
        }
        let mut result = serde_json::from_slice(&bytes)
            .map_err(|_| anyhow::anyhow!("n8n returned malformed JSON"))?;
        attach_execution_id(&mut result, execution_id.as_deref())?;
        attach_correlation_id(&mut result, "audit_id", audit_id.as_deref())?;
        Ok(result)
    }
}

fn attach_execution_id(result: &mut serde_json::Value, id: Option<&str>) -> Result<()> {
    attach_correlation_id(result, "execution_id", id)
}

pub(crate) fn valid_correlation_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_-".contains(&byte))
}

fn attach_correlation_id(
    result: &mut serde_json::Value,
    field: &str,
    id: Option<&str>,
) -> Result<()> {
    let Some(id) = id else {
        return Ok(());
    };
    if !valid_correlation_id(id) {
        bail!("Invalid n8n execution correlation identifier");
    }
    let object = result
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("Invalid n8n response object"))?;
    if object
        .get(field)
        .is_some_and(|value| value.as_str() != Some(id))
    {
        bail!("Conflicting n8n execution correlation identifiers");
    }
    object.insert(field.into(), serde_json::Value::String(id.into()));
    Ok(())
}

fn validate_read_correlation(result: &serde_json::Value, request: &ExecutionRequest) -> Result<()> {
    for (field, expected) in [
        ("request_id", request.id.as_str()),
        ("agent", request.agent.as_str()),
        ("task_id", request.task_id.as_str()),
    ] {
        if result
            .get(field)
            .is_some_and(|value| value.as_str() != Some(expected))
        {
            bail!("Read response scope does not match the dispatched request");
        }
    }
    for field in ["audit_id", "execution_id"] {
        if result
            .get(field)
            .is_some_and(|value| !value.as_str().is_some_and(valid_correlation_id))
        {
            bail!("Invalid read response correlation");
        }
    }
    Ok(())
}

/// Run on a blocking worker. Registry/transport are trusted infrastructure and
/// are never deserialized from model/API input. There is no write endpoint here.
#[allow(clippy::too_many_arguments)]
pub fn invoke_read<G: Guard>(
    host: &AgentHost,
    broker: &ExecutionBroker<G>,
    secrets: &SecretBroker,
    transport: &impl ReadTransport,
    workflow: &ReadWorkflow,
    owner: AgentId,
    task: &str,
    payload: serde_json::Value,
) -> Result<ReadResult> {
    invoke_read_with_id(
        host, broker, secrets, transport, workflow, owner, task, payload, None,
    )
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn invoke_read_with_id<G: Guard>(
    host: &AgentHost,
    broker: &ExecutionBroker<G>,
    secrets: &SecretBroker,
    transport: &impl ReadTransport,
    workflow: &ReadWorkflow,
    owner: AgentId,
    task: &str,
    payload: serde_json::Value,
    request_id: Option<&str>,
) -> Result<ReadResult> {
    host.task(owner, task)?;
    workflow.check_input(owner, &payload)?;
    let mut action =
        ExecutionRequest::new(owner, task, "n8n.read", "n8n.read", &workflow.name, payload);
    if let Some(id) = request_id {
        action.id = uuid::Uuid::parse_str(id)?.to_string();
    }
    match broker.admit(&action)? {
        Admission::Allowed { .. } | Admission::AllowedReadByPolicy => {}
        Admission::NeedsDeepReview { .. } => {
            return Err(crate::security::execution::GuardBlockedAction(action).into())
        }
        Admission::Denied { .. } => {
            bail!("n8n read admission denied by policy, Guard, or approval scope")
        }
    }
    host.audit(&action.id, owner, "n8n_read_started", &workflow.name)?;
    let result = secrets.execute_redacted(
        owner,
        "n8n.read",
        &workflow.credential,
        &action.id,
        |credential| transport.post(&workflow.endpoint, credential, &action),
    );
    let result = result.and_then(|value| {
        validate_read_correlation(&value, &action)?;
        Ok(value)
    });
    let (result, gateway_audit_id, n8n_execution_id) = match result {
        Ok(mut result) if workflow.response_succeeded(&result, &action.payload) => {
            let gateway_audit_id = result
                .get("audit_id")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned);
            let n8n_execution_id = result
                .get("execution_id")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned);
            let object = result
                .as_object_mut()
                .ok_or_else(|| anyhow::anyhow!("Invalid read result schema"))?;
            object.retain(|name, _| workflow.response_fields.contains(name));
            (result, gateway_audit_id, n8n_execution_id)
        }
        _ => {
            host.audit(&action.id, owner, "n8n_read_failed", &workflow.name)?;
            bail!("n8n read workflow failed; no retry was dispatched");
        }
    };
    host.audit(&action.id,owner,"n8n_read_completed",&serde_json::json!({"workflow":workflow.name,"gateway_audit_id":gateway_audit_id,"n8n_execution_id":n8n_execution_id}).to_string())?;
    Ok(ReadResult {
        request_id: action.id,
        agent: owner,
        task_id: task.into(),
        workflow: workflow.name.clone(),
        gateway_audit_id,
        n8n_execution_id,
        result,
    })
}
#[cfg(test)]
pub(crate) mod test_gateway {
    use std::io::{BufRead, Read, Write};

    pub(crate) fn start(
        replies: Vec<serde_json::Value>,
    ) -> (
        String,
        std::thread::JoinHandle<Vec<(String, serde_json::Value)>>,
    ) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}/gateway", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let mut captured = Vec::new();
            for reply in replies {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                    .unwrap();
                let mut reader = std::io::BufReader::new(stream.try_clone().unwrap());
                let mut length = 0;
                let mut authorization = String::new();
                loop {
                    let mut line = String::new();
                    assert!(reader.read_line(&mut line).unwrap() > 0);
                    if line == "\r\n" {
                        break;
                    }
                    if let Some((name, value)) = line.split_once(':') {
                        if name.eq_ignore_ascii_case("content-length") {
                            length = value.trim().parse().unwrap();
                        }
                        if name.eq_ignore_ascii_case("authorization") {
                            authorization = value.trim().to_owned();
                        }
                    }
                }
                assert!(length <= 65_536);
                let mut body = vec![0; length];
                reader.read_exact(&mut body).unwrap();
                captured.push((authorization, serde_json::from_slice(&body).unwrap()));
                let body = reply.to_string();
                write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            }
            captured
        });
        (endpoint, server)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shipped_read_contract_is_valid_and_owner_scoped() {
        let contract: ReadWorkflow =
            toml::from_str(include_str!("../config/n8n-readonly.example.toml")).unwrap();
        contract.validate().unwrap();
        let input = serde_json::json!({"op":"read","mailbox":"INBOX","uid":1});
        assert!(contract.check_input(AgentId::Janet, &input).is_ok());
        assert!(contract.check_input(AgentId::Harley, &input).is_err());
    }
    #[test]
    fn legacy_email_response_requires_matching_scope_and_success_shape() {
        let mut w = workflow();
        w.response_format = ReadResponseFormat::EmailReadonlyV2;
        let payload = serde_json::json!({"op":"read","mailbox":"INBOX","uid":42});
        let valid = serde_json::json!({"read_only":true,"op":"read","mailbox":"INBOX","uid":42,"body":"hello"});
        assert!(w.response_succeeded(&valid, &payload));
        assert!(!workflow().response_succeeded(&valid, &payload));
        for (key, value) in [
            ("ok", serde_json::json!(false)),
            ("ok", serde_json::json!("true")),
            ("error", serde_json::json!("failed")),
            ("read_only", serde_json::json!(false)),
            ("mailbox", serde_json::json!("Sent")),
            ("uid", serde_json::json!(43)),
            ("op", serde_json::json!("attachment_get")),
            ("body", serde_json::Value::Null),
        ] {
            let mut invalid = valid.clone();
            invalid[key] = value;
            assert!(!w.response_succeeded(&invalid, &payload), "{key}");
        }
        let search = serde_json::json!({"op":"search","mailbox":"INBOX"});
        let partial = serde_json::json!({"read_only":true,"op":"search","mailbox":"INBOX","messages":[],"mailbox_errors":[{"error":"failed"}]});
        assert!(w.response_succeeded(&partial, &search));
    }

    #[test]
    fn execution_header_is_bounded_and_cannot_replace_conflicting_body_id() {
        let mut result = serde_json::json!({"read_only":true});
        attach_execution_id(&mut result, Some("12345")).unwrap();
        assert_eq!(result["execution_id"], "12345");
        assert!(attach_execution_id(&mut result, Some("54321")).is_err());
        for id in ["", "invalid\nvalue", "credential:value"] {
            assert!(attach_execution_id(&mut serde_json::json!({}), Some(id)).is_err());
        }
        assert!(attach_execution_id(&mut serde_json::json!({}), Some(&"x".repeat(129))).is_err());
        assert!(attach_execution_id(&mut serde_json::json!({}), None).is_ok());
    }

    fn workflow() -> ReadWorkflow {
        ReadWorkflow {
            name: "email_readonly".into(),
            endpoint: "https://example.com/webhook/email-readonly".into(),
            credential: "email_read".into(),
            response_format: ReadResponseFormat::ExplicitOk,
            allowed_agents: BTreeSet::from(["janet".into()]),
            allowed_operations: BTreeSet::from(["read".into(), "search".into()]),
            fields: BTreeMap::from([
                (
                    "mailbox".into(),
                    FieldRule::Choice {
                        values: BTreeSet::from(["INBOX".into(), "Sent".into()]),
                    },
                ),
                (
                    "uid".into(),
                    FieldRule::Integer {
                        min: 1,
                        max: 2_147_483_647,
                    },
                ),
                ("q".into(), FieldRule::Text { max_bytes: 500 }),
                ("max".into(), FieldRule::Integer { min: 1, max: 30 }),
            ]),
            required_fields: BTreeMap::from([
                (
                    "read".into(),
                    BTreeSet::from(["mailbox".into(), "uid".into()]),
                ),
                (
                    "search".into(),
                    BTreeSet::from(["mailbox".into(), "q".into(), "max".into()]),
                ),
            ]),
            response_fields: BTreeSet::from([
                "ok".into(),
                "body".into(),
                "credential_echo".into(),
                "audit_id".into(),
                "execution_id".into(),
            ]),
        }
    }
    #[test]
    fn read_contract_rejects_peer_access_writes_wildcards_and_connection_input() {
        let w = workflow();
        assert!(w.validate().is_ok());
        assert!(w
            .check_input(
                AgentId::Janet,
                &serde_json::json!({"op":"read","mailbox":"INBOX","uid":1})
            )
            .is_ok());
        for p in [
            serde_json::json!({"op":"email.send"}),
            serde_json::json!({"op":"read","mailbox":"*","uid":1}),
            serde_json::json!({"op":"read","host":"malicious.example","uid":1}),
            serde_json::json!({"op":"search","q":"query","max":1000}),
        ] {
            assert!(w.check_input(AgentId::Janet, &p).is_err());
        }
        assert!(w
            .check_input(AgentId::Harley, &serde_json::json!({"op":"read","uid":1}))
            .is_err());
    }
    #[test]
    fn insecure_endpoints_and_embedded_credentials_are_rejected() {
        let mut w = workflow();
        w.endpoint = "http://example.com".into();
        assert!(w.validate().is_err());
        w.endpoint = "https://secret@example.com".into();
        assert!(w.validate().is_err());
    }
    #[test]
    fn response_scope_and_body_correlation_are_bounded() {
        let request = ExecutionRequest::new(
            AgentId::Janet,
            "task",
            "n8n.read",
            "n8n.read",
            "mail",
            serde_json::json!({}),
        );
        let valid = serde_json::json!({"request_id":request.id,"agent":"janet","task_id":"task","audit_id":"audit-1","execution_id":"exec-1"});
        assert!(validate_read_correlation(&valid, &request).is_ok());
        for (field, value) in [
            ("request_id", serde_json::json!("other")),
            ("agent", serde_json::json!("harley")),
            ("task_id", serde_json::json!("other")),
            ("audit_id", serde_json::json!("x".repeat(129))),
            ("execution_id", serde_json::json!(["not-an-id"])),
        ] {
            let mut invalid = valid.clone();
            invalid[field] = value;
            assert!(validate_read_correlation(&invalid, &request).is_err());
        }
        assert!(!workflow().response_succeeded(
            &serde_json::json!({"ok":true,"error":"failed"}),
            &serde_json::json!({})
        ));
    }

    #[test]
    fn real_read_transport_sends_only_payload_and_preserves_audit_metadata() {
        let (endpoint, server) = super::test_gateway::start(vec![
            serde_json::json!({"ok":true,"audit_id":"audit-1","execution_id":"exec-1"}),
        ]);
        let request = ExecutionRequest::new(
            AgentId::Janet,
            "task",
            "n8n.read",
            "n8n.read",
            "mail",
            serde_json::json!({"op":"read"}),
        );
        let result = HttpsReadTransport::new()
            .unwrap()
            .post(&endpoint, "fixture-webhook", &request)
            .unwrap();
        assert_eq!(result["audit_id"], "audit-1");
        let captured = server.join().unwrap();
        assert_eq!(captured[0].0, "Bearer fixture-webhook");
        assert_eq!(captured[0].1, request.payload);
    }
    struct TestGuard(crate::security::GuardDecision);
    impl Guard for TestGuard {
        fn assess(&self, _: &ExecutionRequest) -> crate::security::GuardDecision {
            self.0
        }
    }
    struct Transport {
        calls: std::cell::Cell<usize>,
        result: serde_json::Value,
    }
    impl ReadTransport for Transport {
        fn post(
            &self,
            _: &str,
            credential: &str,
            _: &ExecutionRequest,
        ) -> Result<serde_json::Value> {
            self.calls.set(self.calls.get() + 1);
            let mut result = self.result.clone();
            result["credential_echo"] = credential.into();
            Ok(result)
        }
    }
    #[test]
    fn live_boundary_redacts_correlates_and_fails_closed_before_transport() {
        let d = tempfile::TempDir::new().unwrap();
        let host = AgentHost::open(&d.path().join("host")).unwrap();
        let task = host
            .create_task(AgentId::Janet, "Read", "Read one message", None)
            .unwrap();
        let root = d.path().join("secrets");
        std::fs::create_dir(&root).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let mut secrets = SecretBroker::open(&root, true).unwrap();
        secrets
            .register(
                "email_read",
                vec![AgentId::Janet],
                vec!["n8n.read".into()],
                "test-webhook-credential",
            )
            .unwrap();
        let transport = Transport {
            calls: std::cell::Cell::new(0),
            result: serde_json::json!({"ok":true,"audit_id":"gateway-audit","execution_id":"n8n-execution","body":"Read evidence"}),
        };
        let denied = ExecutionBroker::with_policy(
            crate::security::ApprovalStore::open(&d.path().join("denied.db")).unwrap(),
            TestGuard(crate::security::GuardDecision::Unavailable),
            crate::security::root::RootPolicy::read_fixture(AgentId::Janet, "email_readonly"),
        );
        let input = serde_json::json!({"op":"read","mailbox":"INBOX","uid":1});
        assert!(invoke_read(
            &host,
            &denied,
            &secrets,
            &transport,
            &workflow(),
            AgentId::Janet,
            &task.id,
            input.clone()
        )
        .is_err());
        assert_eq!(transport.calls.get(), 0);
        let allowed = ExecutionBroker::with_policy(
            crate::security::ApprovalStore::open(&d.path().join("allowed.db")).unwrap(),
            TestGuard(crate::security::GuardDecision::Safe),
            crate::security::root::RootPolicy::read_fixture(AgentId::Janet, "email_readonly"),
        );
        let mut reviewed = workflow();
        reviewed.response_fields.remove("audit_id");
        reviewed.response_fields.remove("execution_id");
        let result = invoke_read(
            &host,
            &allowed,
            &secrets,
            &transport,
            &reviewed,
            AgentId::Janet,
            &task.id,
            input.clone(),
        )
        .unwrap();
        assert_eq!(result.gateway_audit_id.as_deref(), Some("gateway-audit"));
        assert_eq!(result.n8n_execution_id.as_deref(), Some("n8n-execution"));
        assert!(result.result.get("audit_id").is_none());
        assert!(result.result.get("execution_id").is_none());
        assert_eq!(result.result["credential_echo"], "[REDACTED]");
        let failed = Transport {
            calls: std::cell::Cell::new(0),
            result: serde_json::json!({"ok":false,"code":"message_not_found"}),
        };
        assert!(invoke_read(
            &host,
            &allowed,
            &secrets,
            &failed,
            &workflow(),
            AgentId::Janet,
            &task.id,
            input.clone()
        )
        .is_err());
        assert_eq!(failed.calls.get(), 1);
        assert!(invoke_read(
            &host,
            &allowed,
            &secrets,
            &transport,
            &workflow(),
            AgentId::Harley,
            &task.id,
            input
        )
        .is_err());
        assert_eq!(transport.calls.get(), 1);
    }
}
