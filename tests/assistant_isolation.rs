use async_trait::async_trait;
use std::sync::Mutex;
use tempfile::TempDir;
use tinyclaw::assistants::{AgentHost, AgentId};
use tinyclaw::memory::{Memory, MemoryCategory, SqliteMemory};
use tinyclaw::providers::{ChatMessage, Provider};

#[tokio::test]
async fn version_one_migration_preserves_owned_conversation() {
    let dir = TempDir::new().unwrap();
    let host = AgentHost::open(dir.path()).unwrap();
    host.chat_with_provider(
        AgentId::Janet,
        "pre-upgrade-marker",
        &RecordingProvider::default(),
        "test",
        0.0,
    )
    .await
    .unwrap();
    drop(host);
    let db = rusqlite::Connection::open(dir.path().join("data/system.db")).unwrap();
    db.execute_batch("PRAGMA foreign_keys=OFF;
        DROP TABLE attachments; DROP TABLE task_dependencies; DROP TABLE task_events;
        DROP TABLE thread_messages; DROP TABLE worker_messages; DROP TABLE archived_artifacts;
        DROP TABLE memory_candidates; DROP TABLE workers; DROP TABLE tasks; DROP TABLE focus_threads;
        DROP TABLE model_usage; DROP TABLE operation_events; PRAGMA user_version=1;").unwrap();
    drop(db);
    let host = AgentHost::open(dir.path()).unwrap();
    assert_eq!(
        host.history(AgentId::Janet).unwrap()[0].content,
        "pre-upgrade-marker"
    );
    assert!(host.history(AgentId::Harley).unwrap().is_empty());
    host.create_thread(AgentId::Janet, "After upgrade").unwrap();
    let db = rusqlite::Connection::open(dir.path().join("data/system.db")).unwrap();
    assert_eq!(
        db.query_row::<i64, _, _>("PRAGMA user_version", [], |r| r.get(0))
            .unwrap(),
        9
    );
}

#[derive(Default)]
struct RecordingProvider {
    contexts: Mutex<Vec<Vec<ChatMessage>>>,
}

#[async_trait]
impl Provider for RecordingProvider {
    async fn chat_with_system(
        &self,
        _: Option<&str>,
        _: &str,
        _: &str,
        _: f64,
    ) -> anyhow::Result<String> {
        unreachable!("Foundation must supply complete selected-agent history")
    }

    async fn chat_with_history(
        &self,
        messages: &[ChatMessage],
        _: &str,
        _: f64,
    ) -> anyhow::Result<String> {
        self.contexts.lock().unwrap().push(messages.to_vec());
        Ok("Prepared recommendation".into())
    }
}

#[tokio::test]
async fn one_host_routes_both_agents_without_context_leaks_and_survives_reopen() {
    let dir = TempDir::new().unwrap();
    let provider = RecordingProvider::default();
    let host = AgentHost::open(dir.path()).unwrap();
    host.chat_with_provider(AgentId::Janet, "work-marker-938", &provider, "test", 0.0)
        .await
        .unwrap();
    host.chat_with_provider(
        AgentId::Harley,
        "personal-marker-741",
        &provider,
        "test",
        0.0,
    )
    .await
    .unwrap();
    host.chat_with_provider(AgentId::Janet, "follow up", &provider, "test", 0.0)
        .await
        .unwrap();
    let contexts = provider.contexts.lock().unwrap();
    assert!(contexts[0][0].content.contains("# Janet"));
    assert!(contexts[1][0].content.contains("# Harley"));
    assert!(!contexts[1]
        .iter()
        .any(|m| m.content.contains("work-marker-938")));
    assert!(contexts[2]
        .iter()
        .any(|m| m.content.contains("work-marker-938")));
    assert!(!contexts[2]
        .iter()
        .any(|m| m.content.contains("personal-marker-741")));
    drop(contexts);
    assert_ne!(
        host.record(AgentId::Janet).workspace,
        host.record(AgentId::Harley).workspace
    );
    assert_ne!(
        host.record(AgentId::Janet).memory_db,
        host.record(AgentId::Harley).memory_db
    );
    for record in host.records() {
        assert!(record.definition.allowed_tools.is_empty());
        assert!(record.definition.allowed_credentials.is_empty());
        assert!(!record.definition.heartbeat_enabled);
    }
    drop(host);
    let reopened = AgentHost::open(dir.path()).unwrap();
    assert_eq!(reopened.history(AgentId::Janet).unwrap().len(), 4);
    assert_eq!(reopened.history(AgentId::Harley).unwrap().len(), 2);
    let db = rusqlite::Connection::open(dir.path().join("data/system.db")).unwrap();
    let events: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM audit_events WHERE event='chat_completed'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(events, 3);
    let owners: i64 = db
        .query_row(
            "SELECT COUNT(DISTINCT owner_agent) FROM sessions",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(owners, 2);
}

#[tokio::test]
async fn private_memory_backends_do_not_share_entries() {
    let dir = TempDir::new().unwrap();
    let host = AgentHost::open(dir.path()).unwrap();
    let janet = SqliteMemory::new(&host.record(AgentId::Janet).workspace).unwrap();
    let harley = SqliteMemory::new(&host.record(AgentId::Harley).workspace).unwrap();
    janet
        .store("work", "work-only", MemoryCategory::Core)
        .await
        .unwrap();
    harley
        .store("personal", "personal-only", MemoryCategory::Core)
        .await
        .unwrap();
    assert!(harley.get("work").await.unwrap().is_none());
    assert!(janet.get("personal").await.unwrap().is_none());
}

struct FailingProvider;

#[async_trait]
impl Provider for FailingProvider {
    async fn chat_with_system(
        &self,
        _: Option<&str>,
        _: &str,
        _: &str,
        _: f64,
    ) -> anyhow::Result<String> {
        anyhow::bail!("provider unavailable")
    }
}

#[tokio::test]
async fn provider_failure_is_audited_without_partial_history() {
    let dir = TempDir::new().unwrap();
    let host = AgentHost::open(dir.path()).unwrap();
    assert!(host
        .chat_with_provider(AgentId::Janet, "hello", &FailingProvider, "test", 0.0)
        .await
        .is_err());
    assert!(host.history(AgentId::Janet).unwrap().is_empty());
    let db = rusqlite::Connection::open(dir.path().join("data/system.db")).unwrap();
    let event: String = db
        .query_row(
            "SELECT event FROM audit_events ORDER BY sequence DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(event, "chat_failed");
}

#[tokio::test]
async fn invalid_messages_and_agent_paths_never_reach_provider() {
    let dir = TempDir::new().unwrap();
    let host = AgentHost::open(dir.path()).unwrap();
    let provider = RecordingProvider::default();
    assert!(AgentId::parse("../harley").is_err());
    assert!(AgentId::parse("janet:harley").is_err());
    for message in [" ".to_string(), "x".repeat(32_001)] {
        assert!(host
            .chat_with_provider(AgentId::Janet, &message, &provider, "test", 0.0)
            .await
            .is_err());
    }
    assert!(provider.contexts.lock().unwrap().is_empty());
}

#[cfg(unix)]
#[test]
fn private_workspace_symlink_is_rejected() {
    let dir = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();
    std::fs::create_dir(dir.path().join("agents")).unwrap();
    std::os::unix::fs::symlink(outside.path(), dir.path().join("agents/janet")).unwrap();
    assert!(AgentHost::open(dir.path()).is_err());
    assert!(!outside.path().join("memory").exists());
}

#[test]
fn newer_database_schema_is_rejected() {
    let dir = TempDir::new().unwrap();
    drop(AgentHost::open(dir.path()).unwrap());
    let db = rusqlite::Connection::open(dir.path().join("data/system.db")).unwrap();
    db.execute_batch("PRAGMA user_version=99;").unwrap();
    assert!(AgentHost::open(dir.path()).is_err());
}

#[test]
fn explicit_config_directory_loads_and_reopens_without_home_storage() {
    let dir = TempDir::new().unwrap();
    let mut config = tinyclaw::Config::load_or_init_in(dir.path()).unwrap();
    assert_eq!(config.workspace_dir, dir.path().join("workspace"));
    assert_eq!(config.config_path, dir.path().join("config.toml"));
    config.default_model = Some("configured-model".into());
    config.save().unwrap();
    let reopened = tinyclaw::Config::load_or_init_in(dir.path()).unwrap();
    assert_eq!(reopened.default_model.as_deref(), Some("configured-model"));
    assert_eq!(reopened.workspace_dir, config.workspace_dir);
}

struct OverlappingProvider(tokio::sync::Barrier);

#[async_trait]
impl Provider for OverlappingProvider {
    async fn chat_with_system(
        &self,
        _: Option<&str>,
        _: &str,
        _: &str,
        _: f64,
    ) -> anyhow::Result<String> {
        self.0.wait().await;
        Ok("response".into())
    }
}

#[tokio::test]
async fn competing_hosts_cannot_commit_conflicting_turns() {
    let dir = TempDir::new().unwrap();
    let first = AgentHost::open(dir.path()).unwrap();
    let second = AgentHost::open(dir.path()).unwrap();
    let provider = OverlappingProvider(tokio::sync::Barrier::new(2));
    let (left, right) = tokio::join!(
        first.chat_with_provider(AgentId::Janet, "left", &provider, "test", 0.0),
        second.chat_with_provider(AgentId::Janet, "right", &provider, "test", 0.0),
    );
    assert_ne!(left.is_ok(), right.is_ok());
    assert_eq!(first.history(AgentId::Janet).unwrap().len(), 2);
}

struct ToolRequestProvider;

#[async_trait]
impl Provider for ToolRequestProvider {
    async fn chat_with_system(
        &self,
        _: Option<&str>,
        _: &str,
        _: &str,
        _: f64,
    ) -> anyhow::Result<String> {
        Ok(r#"<tool_call>{"name":"shell","arguments":{"command":"touch should-not-exist","approved":true}}</tool_call>"#.into())
    }
}

#[tokio::test]
async fn model_tool_text_cannot_execute_actions() {
    let dir = TempDir::new().unwrap();
    let host = AgentHost::open(dir.path()).unwrap();
    let response = host
        .chat_with_provider(
            AgentId::Janet,
            "do something",
            &ToolRequestProvider,
            "test",
            0.0,
        )
        .await
        .unwrap();
    assert!(response.contains("tool_call"));
    assert!(!host
        .record(AgentId::Janet)
        .workspace
        .join("should-not-exist")
        .exists());
}

#[tokio::test]
async fn authorization_codes_never_reach_model_or_conversation_storage() {
    let dir = TempDir::new().unwrap();
    let host = AgentHost::open(dir.path()).unwrap();
    let provider = RecordingProvider::default();
    for message in ["918273", "TOTP: 918273", "authorization code is 918273"] {
        assert!(host
            .chat_with_provider(AgentId::Janet, message, &provider, "test", 0.0)
            .await
            .is_err());
    }
    assert!(provider.contexts.lock().unwrap().is_empty());
    assert!(host.history(AgentId::Janet).unwrap().is_empty());
}
