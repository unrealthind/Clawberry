//! Host-side diagnostic: never dispatches the assessed action or exposes secrets.
use tinyclaw::assistants::{AgentHost, AgentId};
use tinyclaw::config::Config;
use tinyclaw::security::guard::HuggingFaceGuard;
use tinyclaw::security::{ExecutionRequest, SecretBroker};
fn main() -> anyhow::Result<()> {
    let config = Config::load_or_init()?;
    let root = config.workspace_dir.join("assistants");
    let host = AgentHost::open(&root)?;
    let owner = AgentId::Janet;
    let task = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("Supply an owned Janet task ID"))?;
    host.task(owner, &task)?;
    let request = ExecutionRequest::new(
        owner,
        &task,
        "n8n.read",
        "n8n.read",
        "email_work_readonly",
        serde_json::json!({"op":"recent","mailbox":"INBOX","max":1}),
    );
    let policy = tinyclaw::security::root::RootPolicy::load(std::path::Path::new(
        "/etc/tinyclaw/security/root-policy.toml",
    ))?;
    anyhow::ensure!(
        policy.allows(&request),
        "Host policy does not grant this read"
    );
    let secrets = SecretBroker::open(&root.join("credentials"), true)?;
    let report = secrets.execute_redacted(
        owner,
        "guard.assess",
        "huggingface_guard",
        &request.id,
        |token| {
            let guard = HuggingFaceGuard::new(token.into())?;
            Ok(serde_json::to_value(guard.assess_report(&request))?)
        },
    )?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
