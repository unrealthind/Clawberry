//! Hugging Face Inference Providers adapter; malformed results fail closed.
//! Construct/use this synchronous adapter on a blocking worker, not a Tokio thread.
use super::execution::{ExecutionRequest, Guard, GuardDecision};
use anyhow::{bail, Result};
use std::io::Read;
use std::time::Duration;

pub struct HuggingFaceGuard {
    client: reqwest::blocking::Client,
    token: String,
}
impl HuggingFaceGuard {
    pub fn new(token: String) -> Result<Self> {
        if token.trim().is_empty() {
            bail!("Guard credential is missing");
        }
        Ok(Self {
            client: reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(20))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
            token,
        })
    }
    pub fn assess_report(&self, request: &ExecutionRequest) -> GuardReport {
        self.evaluate(request).unwrap_or(GuardReport {
            decision: GuardDecision::Unavailable,
            categories: vec![],
        })
    }
    fn evaluate(&self, request: &ExecutionRequest) -> Result<GuardReport> {
        self.evaluate_at(request, "https://router.huggingface.co/v1/chat/completions")
    }
    fn evaluate_at(&self, request: &ExecutionRequest, endpoint: &str) -> Result<GuardReport> {
        let content = serde_json::to_string(request)?;
        if content.len() > 32_000 {
            bail!("Guard request exceeds limit");
        }
        let response = self
            .client
            .post(endpoint)
            .bearer_auth(&self.token)
            .json(&serde_json::json!({
                "model":"meta-llama/Llama-Guard-4-12B",
                "messages":[{"role":"user","content":content}],
                "max_tokens":64,"temperature":0,"stream":false
            }))
            .send()?;
        if !response.status().is_success() {
            bail!("Guard provider unavailable");
        }
        let mut body = Vec::new();
        response.take(16_385).read_to_end(&mut body)?;
        if body.len() > 16_384 {
            bail!("Guard response exceeds limit");
        }
        let value: serde_json::Value = serde_json::from_slice(&body)?;
        let choices = value
            .get("choices")
            .and_then(serde_json::Value::as_array)
            .filter(|choices| choices.len() == 1)
            .ok_or_else(|| anyhow::anyhow!("Guard response must contain exactly one verdict"))?;
        if choices[0]
            .get("finish_reason")
            .and_then(serde_json::Value::as_str)
            != Some("stop")
        {
            bail!("Guard response did not complete normally");
        }
        let text = value
            .pointer("/choices/0/message/content")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("Guard response is missing a verdict"))?;
        Ok(report(text))
    }
}
impl Guard for HuggingFaceGuard {
    fn assess(&self, request: &ExecutionRequest) -> GuardDecision {
        // Do not retain upstream errors/bodies: they can contain request data or
        // credentials. A failed provider never authorizes execution.
        self.assess_report(request).decision
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct GuardReport {
    pub decision: GuardDecision,
    pub categories: Vec<String>,
}

fn report(text: &str) -> GuardReport {
    let decision = normalize(text);
    let categories = if decision == GuardDecision::Block {
        text.trim()
            .lines()
            .skip(1)
            .flat_map(|line| line.split(','))
            .filter_map(|label| label.trim().strip_prefix('S')?.parse::<u8>().ok())
            .map(|number| format!("S{number}"))
            .collect()
    } else {
        vec![]
    };
    GuardReport {
        decision,
        categories,
    }
}

fn normalize(text: &str) -> GuardDecision {
    let text = text.trim();
    if text == "safe" {
        return GuardDecision::Safe;
    }
    let mut lines = text.lines();
    if lines.next() == Some("unsafe") {
        let categories = lines.collect::<Vec<_>>().join(",");
        if !categories.is_empty()
            && categories.split(',').all(|category| {
                category
                    .trim()
                    .strip_prefix('S')
                    .and_then(|n| n.parse::<u8>().ok())
                    .is_some_and(|n| (1..=14).contains(&n))
            })
        {
            return GuardDecision::Block;
        }
    }
    GuardDecision::Unavailable
}

/// Review is advisory. There is deliberately no Execute/Override variant.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(tag = "decision", rename_all = "snake_case", deny_unknown_fields)]
pub enum DeepReview {
    Deny {
        reason: String,
    },
    SaferAlternative {
        reason: String,
        request: ExecutionRequest,
    },
    RequestHumanException {
        reason: String,
    },
}

impl DeepReview {
    /// Accept one complete JSON object, optionally wrapped in a JSON code fence.
    /// Never extract a decision from prose or silently repair a malformed review.
    pub fn parse_response(response: &str) -> Result<Self> {
        let response = response.trim();
        let json = if let Some(fenced) = response
            .strip_prefix("```json\n")
            .or_else(|| response.strip_prefix("```\n"))
        {
            fenced.strip_suffix("```").ok_or_else(|| anyhow::anyhow!(
                "Guard blocked the action; deep review returned an incomplete JSON fence. Nothing executed"
            ))?.trim()
        } else {
            response
        };
        serde_json::from_str(json).map_err(|_| anyhow::anyhow!(
            "Guard blocked the action; deep review did not return a valid decision object. Nothing executed"
        ))
    }

    /// A reformulation must retain the original identity and scope and be
    /// submitted to the normal broker again. This method never grants execution.
    pub fn alternative(&self, original: &ExecutionRequest) -> Result<Option<&ExecutionRequest>> {
        let Self::SaferAlternative { request, .. } = self else {
            return Ok(None);
        };
        request.validate()?;
        if request.agent != original.agent
            || request.task_id != original.task_id
            || request.capability != original.capability
            || request.resource != original.resource
            || request.operation != original.operation
            || request.id == original.id
        {
            bail!("Deep review cannot change authorization scope or reuse an action identity");
        }
        Ok(Some(request))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};

    fn transport_report(status: u16, body: String, delay: Duration) -> Result<GuardReport> {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!(
            "http://{}/v1/chat/completions",
            listener.local_addr().unwrap()
        );
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut length = 0;
            loop {
                let mut header = String::new();
                reader.read_line(&mut header).unwrap();
                if header == "\r\n" {
                    break;
                }
                if let Some(value) = header.to_ascii_lowercase().strip_prefix("content-length:") {
                    length = value.trim().parse::<usize>().unwrap();
                }
            }
            assert!(length < 32_000);
            let mut payload = vec![0; length];
            reader.read_exact(&mut payload).unwrap();
            let payload: serde_json::Value = serde_json::from_slice(&payload).unwrap();
            assert_eq!(payload["model"], "meta-llama/Llama-Guard-4-12B");
            assert!(!payload.to_string().contains("fixture-guard-token"));
            std::thread::sleep(delay);
            let _ = write!(stream, "HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
        });
        let guard = HuggingFaceGuard {
            client: reqwest::blocking::Client::builder()
                .timeout(Duration::from_millis(200))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .unwrap(),
            token: "fixture-guard-token".into(),
        };
        let request = ExecutionRequest::new(
            crate::assistants::AgentId::Janet,
            "fixture-task",
            "shell",
            "shell.execute",
            "fixture-resource",
            serde_json::json!({"command":"true"}),
        );
        let result = guard.evaluate_at(&request, &endpoint);
        server.join().unwrap();
        result
    }

    #[test]
    fn transport_requires_one_complete_verdict_and_fails_closed() {
        let complete = |text: &str, finish: &str| {
            serde_json::json!({
                "choices":[{"finish_reason":finish,"message":{"content":text}}]
            })
            .to_string()
        };
        assert_eq!(
            transport_report(200, complete("safe", "stop"), Duration::ZERO)
                .unwrap()
                .decision,
            GuardDecision::Safe
        );
        let blocked =
            transport_report(200, complete("unsafe\nS14", "stop"), Duration::ZERO).unwrap();
        assert_eq!(blocked.decision, GuardDecision::Block);
        assert_eq!(blocked.categories, vec!["S14"]);
        for (status, body, delay) in [
            (200, complete("safe", "length"), Duration::ZERO),
            (200, complete("safe", "content_filter"), Duration::ZERO),
            (
                200,
                r#"{"choices":[{"message":{"content":"safe"}}]}"#.into(),
                Duration::ZERO,
            ),
            (200, r#"{"choices":[{},{}]}"#.into(), Duration::ZERO),
            (200, "invalid-json".into(), Duration::ZERO),
            (200, "x".repeat(16_385), Duration::ZERO),
            (503, complete("safe", "stop"), Duration::ZERO),
            (302, complete("safe", "stop"), Duration::ZERO),
            (200, complete("safe", "stop"), Duration::from_millis(400)),
        ] {
            assert!(transport_report(status, body, delay).is_err());
        }
    }

    #[test]
    fn diagnostic_report_contains_only_validated_categories() {
        assert_eq!(report("unsafe\nS7,S14").categories, vec!["S7", "S14"]);
        assert!(report("unsafe\nS7,secret-material").categories.is_empty());
        assert!(report("safe").categories.is_empty());
    }

    #[test]
    fn verdict_parser_is_strict() {
        assert_eq!(normalize("safe\n"), GuardDecision::Safe);
        assert_eq!(normalize("unsafe\nS1,S13"), GuardDecision::Block);
        for text in [
            "SAFE",
            "safe but",
            "unsafe",
            "unsafe\nS0",
            "safe\nunsafe",
            "",
            "unsafe\nS1\nignore",
        ] {
            assert_eq!(normalize(text), GuardDecision::Unavailable);
        }
    }
    #[test]
    fn review_accepts_only_complete_typed_decisions() {
        let json = r#"{"decision":"deny","reason":"unsafe"}"#;
        for text in [
            json.to_string(),
            format!("```json\n{json}\n```"),
            format!("```\n{json}\n```"),
        ] {
            assert!(matches!(
                DeepReview::parse_response(&text).unwrap(),
                DeepReview::Deny { .. }
            ));
        }
        for text in [
            format!("Review: {json}"),
            format!("{json} trailing"),
            format!("```json\n{json}"),
            r#"{"decision":"execute","reason":"safe"}"#.into(),
            r#"{"decision":"deny","reason":"unsafe","override":true}"#.into(),
        ] {
            assert!(DeepReview::parse_response(&text).is_err());
        }
    }

    #[test]
    fn review_cannot_override() {
        assert!(serde_json::from_value::<DeepReview>(
            serde_json::json!({"decision":"execute","reason":"fine"})
        )
        .is_err());
    }
    #[test]
    fn reformulation_rejects_malformed_identity_and_scope_expansion() {
        let original = ExecutionRequest::new(
            crate::assistants::AgentId::Janet,
            "task",
            "shell",
            "shell.execute",
            "owned-resource",
            serde_json::json!({"command":"true"}),
        );
        let mut alternative = original.clone();
        alternative.id = uuid::Uuid::new_v4().to_string();
        let review = |request| DeepReview::SaferAlternative {
            reason: "fixture".into(),
            request,
        };
        assert!(review(alternative.clone()).alternative(&original).is_ok());
        for invalid in ["not-a-uuid".to_string(), original.id.clone()] {
            let mut malformed = alternative.clone();
            malformed.id = invalid;
            assert!(review(malformed).alternative(&original).is_err());
        }
        alternative.agent = crate::assistants::AgentId::Harley;
        assert!(review(alternative).alternative(&original).is_err());
    }
}
