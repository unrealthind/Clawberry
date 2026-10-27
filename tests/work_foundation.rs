use tempfile::TempDir;
use tinyclaw::assistants::models::{ModelSettings, Profile};
use tinyclaw::assistants::work::TaskStatus;
use tinyclaw::assistants::workers::{ContextItem, WorkerResult, WorkerSpec};
use tinyclaw::assistants::{AgentHost, AgentId};

#[test]
fn durable_work_enforces_ownership_dependencies_and_terminal_states() {
    let dir = TempDir::new().unwrap();
    let host = AgentHost::open(dir.path()).unwrap();
    let owner = AgentId::Janet;
    let other = AgentId::Harley;
    let thread = host.create_thread(owner, "Permit investigation").unwrap();
    let first = host
        .create_task(
            owner,
            "Read comments",
            "Inspect supplied evidence",
            Some(&thread.id),
        )
        .unwrap();
    let second = host
        .create_task(
            owner,
            "Prepare recommendation",
            "Distill findings",
            Some(&thread.id),
        )
        .unwrap();
    let personal = host
        .create_task(other, "Personal plan", "Private", None)
        .unwrap();
    assert!(host.thread(other, &thread.id).is_err());
    assert!(host.task(other, &first.id).is_err());
    assert!(host
        .create_task(other, "Spoof", "Attempt", Some(&thread.id))
        .is_err());
    assert!(host.add_dependency(owner, &first.id, &personal.id).is_err());
    host.add_dependency(owner, &second.id, &first.id).unwrap();
    assert!(host.add_dependency(owner, &first.id, &second.id).is_err());
    assert!(host
        .set_task_status(owner, &second.id, TaskStatus::Completed)
        .is_err());
    assert!(host
        .set_task_status(owner, &first.id, TaskStatus::Executing)
        .is_err());
    host.set_task_status(owner, &first.id, TaskStatus::Completed)
        .unwrap();
    assert!(host
        .set_task_status(owner, &first.id, TaskStatus::Queued)
        .is_err());
    host.set_task_status(owner, &second.id, TaskStatus::Completed)
        .unwrap();
    host.summarize_thread(
        owner,
        &thread.id,
        "Two comments reviewed; recommendation ready",
    )
    .unwrap();
    let attachment = host
        .put_attachment(
            owner,
            &thread.id,
            Some(&second.id),
            "comments.txt",
            b"source material",
        )
        .unwrap();
    assert_eq!(attachment.checksum.len(), 64);
    assert!(host
        .put_attachment(other, &thread.id, None, "leak.txt", b"no")
        .is_err());
    assert!(host
        .put_attachment(owner, &thread.id, None, "../escape", b"no")
        .is_err());
    assert!(host
        .put_attachment(owner, &thread.id, Some(&personal.id), "wrong.txt", b"no")
        .is_err());
    assert!(host.history(owner).unwrap().is_empty());
    drop(host);
    let restored = AgentHost::open(dir.path()).unwrap();
    assert_eq!(restored.tasks(owner).unwrap().len(), 2);
    assert_eq!(restored.tasks(other).unwrap().len(), 1);
    assert_eq!(
        restored.thread(owner, &thread.id).unwrap().summary,
        "Two comments reviewed; recommendation ready"
    );
    assert_eq!(restored.attachments(owner, &thread.id).unwrap().len(), 1);
    assert_eq!(
        restored.task(owner, &second.id).unwrap().status,
        TaskStatus::Completed
    );
}

#[test]
fn worker_result_rejects_fabricated_sources_unsafe_artifacts_and_confidence() {
    let spec = WorkerSpec {
        task_id: "fixture".into(),
        role: "reviewer".into(),
        objective: "Review selected context".into(),
        context: vec![ContextItem {
            source_ref: "doc-1".into(),
            source_type: "file".into(),
            content: "Selected content".into(),
        }],
        profile: Profile::Fast,
        budget_microusd: 100_000,
        timeout_secs: 10,
        allowed_tools: vec![],
        allowed_credentials: vec![],
        retry_deep: false,
    };
    let valid = serde_json::json!({"summary":"Reviewed", "findings":["Evidence found"], "confidence":0.8,
        "sources":[{"source_ref":"doc-1","source_type":"file"}],
        "artifacts":[{"filename":"review.md","content":"Distilled evidence"}],
        "memory_candidates":[{"content":"A useful fact","category":"project","confidence":0.8,"source_ref":"doc-1","sensitivity":"ordinary"}]});
    serde_json::from_value::<WorkerResult>(valid.clone())
        .unwrap()
        .validate(&spec)
        .unwrap();
    for (pointer, value) in [
        (
            "/sources/0/source_ref",
            serde_json::json!("unprovided-private-db"),
        ),
        ("/artifacts/0/filename", serde_json::json!("../escape")),
        ("/confidence", serde_json::json!(1.2)),
        (
            "/memory_candidates/0/source_ref",
            serde_json::json!("invented"),
        ),
        (
            "/memory_candidates/0/sensitivity",
            serde_json::json!("ignore-policy"),
        ),
    ] {
        let mut invalid = valid.clone();
        *invalid.pointer_mut(pointer).unwrap() = value;
        assert!(serde_json::from_value::<WorkerResult>(invalid)
            .unwrap()
            .validate(&spec)
            .is_err());
    }
}

#[test]
fn profile_configuration_is_strict_and_provider_independent() {
    let dir = TempDir::new().unwrap();
    let config = tinyclaw::Config::load_or_init_in(dir.path()).unwrap();
    let settings = ModelSettings::load(&config).unwrap();
    assert_eq!(settings.profiles["normal"].model, "claude-sonnet-5-5");
    assert_eq!(settings.profiles["fast"].model, "claude-haiku-4-5-20251001");
    assert_eq!(settings.profiles["deep"].model, "claude-opus-5-5");
    let mut alternate = settings;
    alternate.profiles.get_mut("normal").unwrap().provider = "openai".into();
    alternate.profiles.get_mut("normal").unwrap().model = "administrator-alias".into();
    std::fs::write(
        dir.path().join("model-profiles.toml"),
        toml::to_string(&alternate).unwrap(),
    )
    .unwrap();
    assert_eq!(
        ModelSettings::load(&config).unwrap().profiles["normal"].model,
        "administrator-alias"
    );
    alternate.profiles.remove("deep");
    std::fs::write(
        dir.path().join("model-profiles.toml"),
        toml::to_string(&alternate).unwrap(),
    )
    .unwrap();
    assert!(ModelSettings::load(&config).is_err());
    assert!(Profile::parse("provider-model-name").is_err());
}

#[test]
fn restart_archives_interrupted_workers_and_preserves_pending_cancellation() {
    let dir = TempDir::new().unwrap();
    let host = AgentHost::open(dir.path()).unwrap();
    let task = host
        .create_task(
            AgentId::Janet,
            "Interrupted review",
            "Retain failure evidence",
            None,
        )
        .unwrap();
    let workspace = host.record(AgentId::Janet).workspace.clone();
    drop(host);
    let db = rusqlite::Connection::open(dir.path().join("data/system.db")).unwrap();
    let spec = serde_json::json!({"task_id":task.id,"role":"reviewer","objective":"Review evidence","context":[],"profile":"normal","budget_microusd":100000,"timeout_secs":10});
    let interrupted = uuid::Uuid::new_v4().to_string();
    let cancelled = uuid::Uuid::new_v4().to_string();
    for (id, status) in [(&interrupted, "running"), (&cancelled, "archive_pending")] {
        std::fs::create_dir_all(workspace.join("workers").join(id)).unwrap();
        db.execute("INSERT INTO workers(id,owner_agent,task_id,spec,status,lease_expires_at) VALUES (?1,'janet',?2,?3,?4,0)",rusqlite::params![id,task.id,spec.to_string(),status]).unwrap();
    }
    db.execute("INSERT INTO worker_messages(worker_id,owner_agent,sender,recipient,message_type,body) VALUES (?1,'janet',?1,'janet','result',?2)",rusqlite::params![cancelled,r#"{"status":"cancelled","result":null,"error":"Owner cancelled"}"#]).unwrap();
    drop(db);
    let restored = AgentHost::open(dir.path()).unwrap();
    assert_eq!(
        restored.task(AgentId::Janet, &task.id).unwrap().status,
        TaskStatus::Blocked
    );
    assert_eq!(
        restored
            .worker(AgentId::Janet, &interrupted)
            .unwrap()
            .status,
        "interrupted"
    );
    assert_eq!(
        restored.worker(AgentId::Janet, &cancelled).unwrap().status,
        "cancelled"
    );
    assert!(restored
        .archive_file(AgentId::Janet, &interrupted, "summary.md")
        .unwrap()
        .windows(11)
        .any(|bytes| bytes == b"interrupted"));
    assert!(restored
        .archive_file(AgentId::Harley, &interrupted, "summary.md")
        .is_err());
    assert!(!workspace.join("workers").join(interrupted).exists());
    assert!(!workspace.join("workers").join(cancelled).exists());
    assert_eq!(restored.recover_workers(AgentId::Janet).unwrap(), 0);
}

#[test]
fn task_edits_are_atomic_owned_audited_and_reject_terminal_changes() {
    let dir = TempDir::new().unwrap();
    let host = AgentHost::open(dir.path()).unwrap();
    let task = host
        .create_task(AgentId::Janet, "Original", "Description", None)
        .unwrap();
    let thread = host.create_thread(AgentId::Janet, "Work").unwrap();
    let personal = host.create_thread(AgentId::Harley, "Personal").unwrap();
    assert!(host
        .edit_task(AgentId::Harley, &task.id, "Leak", "No", None, "high", None)
        .is_err());
    assert!(host
        .edit_task(
            AgentId::Janet,
            &task.id,
            "Bad thread",
            "No",
            Some(&personal.id),
            "high",
            None
        )
        .is_err());
    assert!(host
        .edit_task(
            AgentId::Janet,
            &task.id,
            "Bad date",
            "No",
            None,
            "high",
            Some("2026-10-23")
        )
        .is_err());
    assert!(host
        .edit_task(
            AgentId::Janet,
            &task.id,
            "Bad priority",
            "No",
            None,
            "arbitrary",
            None
        )
        .is_err());
    assert_eq!(
        host.task(AgentId::Janet, &task.id).unwrap().title,
        "Original"
    );
    host.edit_task(
        AgentId::Janet,
        &task.id,
        "Reviewed",
        "New description",
        Some(&thread.id),
        "urgent",
        Some("2026-10-23T23:59:59-06:00"),
    )
    .unwrap();
    let updated = host.task(AgentId::Janet, &task.id).unwrap();
    assert_eq!(updated.title, "Reviewed");
    assert_eq!(updated.priority, "urgent");
    assert_eq!(updated.thread_id.as_deref(), Some(thread.id.as_str()));
    let detail = host.task_details(AgentId::Janet, &task.id).unwrap();
    assert!(detail["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(|event| event["event"] == "edited"
            && event["detail"]
                .as_str()
                .unwrap()
                .contains("New description")));
    host.set_task_status(AgentId::Janet, &task.id, TaskStatus::Completed)
        .unwrap();
    assert!(host
        .edit_task(
            AgentId::Janet,
            &task.id,
            "Rewrite",
            "No",
            None,
            "normal",
            None
        )
        .is_err());
    assert_eq!(
        host.task(AgentId::Janet, &task.id).unwrap().title,
        "Reviewed"
    );
}

#[test]
fn decisions_are_durable_idempotent_owned_and_do_not_authorize_tasks() {
    let dir = TempDir::new().unwrap();
    let host = AgentHost::open(dir.path()).unwrap();
    let thread = host
        .create_thread(AgentId::Janet, "Reviewed evidence")
        .unwrap();
    let task = host
        .create_task(AgentId::Janet, "Prepare", "Review", Some(&thread.id))
        .unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    let content = "Keep the release focused";
    for _ in 0..2 {
        assert_eq!(
            host.record_decision(AgentId::Janet, &thread.id, &id, content)
                .unwrap()
                .len(),
            1
        );
    }
    assert!(host
        .record_decision(AgentId::Janet, &thread.id, &id, "Changed")
        .is_err());
    assert!(host
        .record_decision(
            AgentId::Harley,
            &thread.id,
            &uuid::Uuid::new_v4().to_string(),
            content
        )
        .is_err());
    assert!(host.decisions(AgentId::Harley, &thread.id).is_err());
    let other = host
        .create_thread(AgentId::Janet, "Separate scope")
        .unwrap();
    assert!(host
        .record_decision(AgentId::Janet, &other.id, &id, content)
        .is_err());
    assert!(host
        .record_decision(AgentId::Janet, &thread.id, "invalid", content)
        .is_err());
    assert!(host
        .record_decision(
            AgentId::Janet,
            &thread.id,
            &uuid::Uuid::new_v4().to_string(),
            " "
        )
        .is_err());
    drop(host);
    let restored = AgentHost::open(dir.path()).unwrap();
    assert_eq!(
        restored.decisions(AgentId::Janet, &thread.id).unwrap()[0].content,
        content
    );
    assert_eq!(
        restored.task(AgentId::Janet, &task.id).unwrap().status,
        TaskStatus::Queued
    );
    let db = rusqlite::Connection::open(dir.path().join("data/system.db")).unwrap();
    let count: i64 = db
        .query_row(
            "SELECT count(*) FROM audit_events WHERE event='decision_recorded'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
}
