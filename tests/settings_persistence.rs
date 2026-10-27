use tempfile::TempDir;
use tinyclaw::{
    assistants::{
        settings::{Skill, SkillFile, Versioned},
        AgentHost, AgentId,
    },
    config::Config,
};
#[test]
fn settings_are_durable_owned_versioned_and_nonsecret() {
    let dir = TempDir::new().unwrap();
    let host = AgentHost::open(dir.path()).unwrap();
    let config = Config {
        config_path: dir.path().join("config.toml"),
        default_provider: Some("anthropic".into()),
        default_model: Some("claude-sonnet-5-5".into()),
        ..Config::default()
    };
    let mut app = host.global_settings(&config).unwrap();
    app.value.preferences.theme = "light".into();
    app.value.models.agent_daily_microusd = 1_000_000;
    let old = app.clone();
    let saved = host.save_global_settings(app).unwrap();
    assert_eq!(saved.revision, 1);
    assert!(host.save_global_settings(old).is_err());
    let mut janet = host.agent_settings(AgentId::Janet).unwrap();
    janet.value.soul = "Work guidance marker".into();
    janet.value.memory_notes = "Private work note".into();
    janet.value.skills = vec![Skill {
        id: "brief".into(),
        name: "Brief".into(),
        description: String::new(),
        files: vec![SkillFile {
            path: "SKILL.md".into(),
            content: "Summarize selected evidence".into(),
        }],
    }];
    host.save_agent_settings(AgentId::Janet, janet).unwrap();
    assert!(!host
        .agent_settings(AgentId::Harley)
        .unwrap()
        .value
        .soul
        .contains("Work guidance marker"));
    drop(host);
    // Recreate the prior v8 layout: upgrade must retain saved owner/global settings.
    let previous = rusqlite::Connection::open(dir.path().join("data/system.db")).unwrap();
    previous
        .execute_batch("DROP TABLE thread_decisions; PRAGMA user_version=8;")
        .unwrap();
    drop(previous);
    let host = AgentHost::open(dir.path()).unwrap();
    assert_eq!(
        host.global_settings(&config)
            .unwrap()
            .value
            .preferences
            .theme,
        "light"
    );
    assert_eq!(
        host.agent_settings(AgentId::Janet)
            .unwrap()
            .value
            .memory_notes,
        "Private work note"
    );
    let mut invalid = host.agent_settings(AgentId::Janet).unwrap();
    invalid.value.skills[0].files[0].path = "../secret".into();
    assert!(host.save_agent_settings(AgentId::Janet, invalid).is_err());
    let mut invalid = host.global_settings(&config).unwrap();
    invalid
        .value
        .models
        .profiles
        .get_mut("normal")
        .unwrap()
        .provider = "custom:http://evil".into();
    assert!(host.save_global_settings(invalid).is_err());
    assert!(serde_json::from_value::<Versioned<tinyclaw::assistants::settings::AgentSettings>>(serde_json::json!({"revision":0,"value":{"soul":"x","memory_notes":"y","default_profile":"normal","skills":[],"api_key":"should-not-be-accepted"}})).is_err());
    let db = rusqlite::Connection::open(dir.path().join("data/system.db")).unwrap();
    let count: i64 = db
        .query_row("SELECT count(*) FROM operation_events", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0);
    let detail: String = db
        .query_row(
            "SELECT group_concat(detail) FROM audit_events WHERE event='settings_saved'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(!detail.contains("Private work note"));
}

#[test]
fn provider_verification_requires_each_current_profile_not_only_the_model() {
    let dir = TempDir::new().unwrap();
    let host = AgentHost::open(dir.path()).unwrap();
    let config = Config {
        config_path: dir.path().join("config.toml"),
        default_provider: Some("anthropic".into()),
        default_model: Some("claude-sonnet-5-5".into()),
        api_key: Some("fixture-not-a-live-key".into()),
        ..Config::default()
    };
    let mut global = host.global_settings(&config).unwrap();
    for profile in global.value.models.profiles.values_mut() {
        profile.model = "claude-sonnet-5-5".into();
    }
    host.save_global_settings(global).unwrap();
    let db = rusqlite::Connection::open(dir.path().join("data/system.db")).unwrap();
    for profile in ["normal", "fast", "deep"] {
        db.execute("INSERT INTO model_usage(id,owner_agent,profile,provider,model,cost_microusd,reserved_microusd,status) VALUES (?1,'janet',?1,'anthropic','claude-sonnet-5-5',0,0,'completed')",[profile]).unwrap();
        let status = host.runtime_status(&config).unwrap();
        assert_eq!(status["live_verified"], profile == "deep");
    }
}
