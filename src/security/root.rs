//! Administrator-owned execution allowlist. Model safety is not authorization.
use super::execution::{ExecutionRequest, ROOT_DENY_OPERATIONS};
use crate::assistants::AgentId;
use anyhow::{bail, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RootPolicy {
    #[serde(default)]
    rules: Vec<Rule>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Rule {
    agent: AgentId,
    capability: String,
    operation: String,
    /// Exact logical resource; no wildcard, path-prefix, or purpose matching.
    resource: String,
    #[serde(default)]
    autonomous_read: bool,
}

impl RootPolicy {
    /// Fail closed on non-Unix platforms and writable/symlinked ancestors.
    /// Run the orchestration service unprivileged: root can always rewrite policy.
    pub fn load(path: &Path) -> Result<Self> {
        verify_administrator_file(path)?;
        let policy: Self = toml::from_str(&std::fs::read_to_string(path)?)?;
        policy.validate()?;
        Ok(policy)
    }
}

pub(crate) fn verify_administrator_file(path: &Path) -> Result<()> {
    if !path.is_absolute() {
        bail!("Root policy requires an absolute administrator path");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        for component in path.ancestors() {
            let metadata = component.symlink_metadata()?;
            if metadata.file_type().is_symlink()
                || metadata.uid() != 0
                || metadata.mode() & 0o022 != 0
            {
                bail!("Root policy and ancestors must be root-owned, non-symlinked and not group/world writable");
            }
        }
        if !path.metadata()?.is_file() || path.metadata()?.len() > 65_536 {
            bail!("Invalid root policy file");
        }
        #[cfg(target_os = "linux")]
        {
            let status = std::fs::read_to_string("/proc/self/status")?;
            let effective = status
                .lines()
                .find(|line| line.starts_with("Uid:"))
                .and_then(|line| line.split_whitespace().nth(2));
            if effective.is_none() || effective == Some("0") {
                bail!("Execution runtime must run as a non-root user");
            }
        }
        Ok(())
    }
    #[cfg(not(unix))]
    {
        bail!("Host-owned execution policy requires Unix permissions")
    }
}

impl RootPolicy {
    fn validate(&self) -> Result<()> {
        for rule in &self.rules {
            if rule.autonomous_read && rule.operation != "n8n.read" {
                bail!("Only reviewed n8n read contracts can use autonomous admission");
            }
            if [&rule.capability, &rule.operation, &rule.resource]
                .iter()
                .any(|v| v.trim().is_empty())
                || ROOT_DENY_OPERATIONS.contains(&rule.operation.as_str())
            {
                bail!("Root policy cannot grant an invalid or immutable deny operation");
            }
            // Shells are networkless and have no host mounts. External writes
            // have a separate n8n/TOTP admission path, never this approval store.
            if ![
                "shell.execute",
                "shell.read_file",
                "shell.write_file",
                "shell.list",
                "package.search",
                "package.install",
                "package.remove",
                "package.list",
                "media.process",
                "n8n.read",
                "n8n.write",
            ]
            .contains(&rule.operation.as_str())
            {
                bail!("Unsupported operation in execution policy");
            }
        }
        Ok(())
    }

    pub fn allows(&self, request: &ExecutionRequest) -> bool {
        !ROOT_DENY_OPERATIONS.contains(&request.operation.as_str())
            && self.rules.iter().any(|rule| {
                rule.agent == request.agent
                    && rule.capability == request.capability
                    && rule.operation == request.operation
                    && rule.resource == request.resource
            })
    }
    pub(super) fn permits_autonomous_read(&self, request: &ExecutionRequest) -> bool {
        request.operation == "n8n.read"
            && self.rules.iter().any(|rule| {
                rule.autonomous_read
                    && rule.agent == request.agent
                    && rule.capability == request.capability
                    && rule.operation == request.operation
                    && rule.resource == request.resource
            })
    }

    #[cfg(test)]
    pub(crate) fn fixture(
        agent: AgentId,
        capability: &str,
        operation: &str,
        resource: &str,
    ) -> Self {
        Self {
            rules: vec![Rule {
                agent,
                capability: capability.into(),
                operation: operation.into(),
                resource: resource.into(),
                autonomous_read: false,
            }],
        }
    }
    #[cfg(test)]
    pub(crate) fn read_fixture(owner: AgentId, resource: &str) -> Self {
        Self {
            rules: vec![Rule {
                agent: owner,
                capability: "n8n.read".into(),
                operation: "n8n.read".into(),
                resource: resource.into(),
                autonomous_read: true,
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shipped_policy_is_deny_all() {
        let policy: RootPolicy =
            toml::from_str(include_str!("../../config/root-policy.example.toml")).unwrap();
        policy.validate().unwrap();
        assert!(policy.rules.is_empty());
    }
    #[test]
    fn unknown_and_cross_agent_actions_are_denied() {
        let policy = RootPolicy::fixture(AgentId::Janet, "shell", "shell.execute", "task:1");
        let mut r = ExecutionRequest::new(
            AgentId::Janet,
            "1",
            "shell",
            "shell.execute",
            "task:1",
            serde_json::json!({}),
        );
        assert!(policy.allows(&r));
        r.agent = AgentId::Harley;
        assert!(!policy.allows(&r));
        r.agent = AgentId::Janet;
        r.operation = "n8n.write".into();
        assert!(!policy.allows(&r));
        assert!(policy.validate().is_ok());
    }
    #[test]
    fn tampered_or_runtime_owned_policy_is_rejected() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("policy.toml");
        std::fs::write(&path, "rules = []").unwrap();
        assert!(RootPolicy::load(&path).is_err());
    }
}
