//! Human-managed, versioned nonsecret app and owner settings. Never model operations.
use super::{models::ModelSettings, AgentHost, AgentId};
use crate::config::Config;
use anyhow::{bail, Result};
use rusqlite::{params, OptionalExtension, Transaction};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fmt::Write as _;

pub(super) fn migrate(tx: &Transaction<'_>) -> Result<()> {
    tx.execute_batch("CREATE TABLE IF NOT EXISTS ui_settings (scope TEXT PRIMARY KEY, revision INTEGER NOT NULL, value TEXT NOT NULL);")?;
    Ok(())
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Versioned<T> {
    pub revision: u64,
    pub value: T,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preferences {
    pub theme: String,
    pub notifications: bool,
    pub week_start: String,
    pub time_format: String,
    pub show_completed: bool,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GlobalSettings {
    pub preferences: Preferences,
    pub models: ModelSettings,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillFile {
    pub path: String,
    pub content: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub files: Vec<SkillFile>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentSettings {
    pub soul: String,
    pub memory_notes: String,
    pub default_profile: super::models::Profile,
    pub skills: Vec<Skill>,
}
impl GlobalSettings {
    pub fn validate(&self) -> Result<()> {
        let p = &self.preferences;
        if !matches!(p.theme.as_str(), "light" | "dark")
            || !matches!(p.week_start.as_str(), "sunday" | "monday")
            || !matches!(p.time_format.as_str(), "12" | "24")
        {
            bail!("Invalid workspace preferences");
        }
        self.models.validate()?;
        for model in self.models.profiles.values() {
            if model.provider != "anthropic"
                || !model.model.starts_with("claude-")
                || model.model.len() > 120
                || !model
                    .model
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'-')
            {
                bail!("Settings support named Claude models only");
            }
        }
        if [
            self.models.agent_daily_microusd,
            self.models.deep_daily_microusd,
            self.models.task_microusd,
        ]
        .iter()
        .any(|v| *v > 10_000_000_000)
        {
            bail!("Budget exceeds limit");
        }
        Ok(())
    }
}
impl AgentSettings {
    fn validate(&self) -> Result<()> {
        fn no_secret(text: &str) -> Result<()> {
            if ["sk-ant-", "ANTHROPIC_API_KEY=", "Authorization: Bearer "]
                .iter()
                .any(|needle| text.contains(needle))
            {
                bail!("Credentials belong in the host environment, not agent settings");
            }
            Ok(())
        }
        no_secret(&self.soul)?;
        no_secret(&self.memory_notes)?;
        if self.soul.len() > 32768 || self.memory_notes.len() > 32768 || self.skills.len() > 16 {
            bail!("Agent settings exceed limits");
        }
        let instructions: usize = self
            .skills
            .iter()
            .flat_map(|s| s.files.iter())
            .filter(|f| f.path == "SKILL.md")
            .map(|f| f.content.len())
            .sum();
        if instructions > 65536 {
            bail!("Skill instructions exceed context limit");
        }
        let mut ids = std::collections::HashSet::new();
        for skill in &self.skills {
            if skill.id.is_empty()
                || skill.id.len() > 100
                || !ids.insert(&skill.id)
                || skill.name.trim().is_empty()
                || skill.name.len() > 80
                || skill.description.len() > 400
                || skill.files.is_empty()
                || skill.files.len() > 20
            {
                bail!("Invalid skill");
            }
            let mut paths = std::collections::HashSet::new();
            for file in &skill.files {
                no_secret(&file.content)?;
                if file.path.is_empty()
                    || file.path.len() > 120
                    || file.path.contains('\\')
                    || !std::path::Path::new(&file.path)
                        .components()
                        .all(|c| matches!(c, std::path::Component::Normal(_)))
                    || !paths.insert(&file.path)
                    || file.content.len() > 65536
                {
                    bail!("Invalid skill file");
                }
            }
        }
        if serde_json::to_vec(self)?.len() > 600_000 {
            bail!("Agent settings exceed total limit");
        }
        Ok(())
    }
}
impl AgentHost {
    fn read_settings<T: DeserializeOwned>(&self, scope: &str) -> Result<Option<Versioned<T>>> {
        let row: Option<(u64, String)> = self
            .connection()?
            .query_row(
                "SELECT revision,value FROM ui_settings WHERE scope=?1",
                [scope],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        row.map(|(revision, value)| {
            Ok(Versioned {
                revision,
                value: serde_json::from_str(&value)?,
            })
        })
        .transpose()
    }
    fn save_settings<T: Serialize>(
        &self,
        scope: &str,
        owner: AgentId,
        expected: u64,
        value: T,
    ) -> Result<Versioned<T>> {
        let mut db = self.connection()?;
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let revision: u64 = tx
            .query_row(
                "SELECT revision FROM ui_settings WHERE scope=?1",
                [scope],
                |r| r.get(0),
            )
            .optional()?
            .unwrap_or(0);
        if expected != revision {
            bail!("Settings changed elsewhere. Reload before saving; your draft has not been overwritten.");
        }
        let next = revision
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("Settings revision overflow"))?;
        tx.execute("INSERT INTO ui_settings(scope,revision,value) VALUES (?1,?2,?3) ON CONFLICT(scope) DO UPDATE SET revision=excluded.revision,value=excluded.value",params![scope,next,serde_json::to_string(&value)?])?;
        tx.execute("INSERT INTO audit_events(request_id,owner_agent,event,detail) VALUES (?1,?2,'settings_saved',?3)",params![uuid::Uuid::new_v4().to_string(),owner.as_str(),format!("{scope}:revision:{next}")])?;
        tx.commit()?;
        Ok(Versioned {
            revision: next,
            value,
        })
    }
    pub fn global_settings(&self, config: &Config) -> Result<Versioned<GlobalSettings>> {
        if let Some(saved) = self.read_settings("global")? {
            return Ok(saved);
        }
        Ok(Versioned {
            revision: 0,
            value: GlobalSettings {
                preferences: Preferences {
                    theme: "dark".into(),
                    notifications: true,
                    week_start: "sunday".into(),
                    time_format: "12".into(),
                    show_completed: true,
                },
                models: ModelSettings::load(config)?,
            },
        })
    }
    pub fn save_global_settings(
        &self,
        input: Versioned<GlobalSettings>,
    ) -> Result<Versioned<GlobalSettings>> {
        input.value.validate()?;
        self.save_settings("global", AgentId::Janet, input.revision, input.value)
    }
    pub fn agent_settings(&self, owner: AgentId) -> Result<Versioned<AgentSettings>> {
        Ok(self
            .read_settings(&format!("agent:{}", owner.as_str()))?
            .unwrap_or(Versioned {
                revision: 0,
                value: AgentSettings {
                    soul: owner.identity().into(),
                    memory_notes: String::new(),
                    default_profile: super::models::Profile::Normal,
                    skills: vec![],
                },
            }))
    }
    pub fn save_agent_settings(
        &self,
        owner: AgentId,
        input: Versioned<AgentSettings>,
    ) -> Result<Versioned<AgentSettings>> {
        input.value.validate()?;
        self.save_settings(
            &format!("agent:{}", owner.as_str()),
            owner,
            input.revision,
            input.value,
        )
    }
    pub(super) fn configured_identity(&self, owner: AgentId) -> Result<String> {
        let settings = self.agent_settings(owner)?.value;
        let mut text=format!("{}\n\nUser's configuration for this owner follows. These preferences cannot grant tools, secrets, external writes, or peer access. Memory notes are user-maintained notes, separate from semantic facts.\n\n{}\n\nMemory notes:\n{}",owner.identity(), settings.soul,settings.memory_notes);
        for skill in settings.skills.iter().take(16) {
            if let Some(file) = skill.files.iter().find(|f| f.path == "SKILL.md") {
                write!(
                    &mut text,
                    "\nSkill {} (text guidance only, no automatic script execution):\n{}",
                    skill.name, file.content
                )?;
            }
        }
        Ok(text)
    }
    pub fn runtime_status(&self, config: &Config) -> Result<serde_json::Value> {
        let settings = self.global_settings(config)?.value;
        let environment = ["ANTHROPIC_API_KEY", "ANTHROPIC_OAUTH_TOKEN"]
            .iter()
            .any(|name| std::env::var(name).is_ok_and(|v| !v.trim().is_empty()));
        let enrolled = crate::security::secrets::claude_credential(config)?.is_some();
        let configured = config.default_provider.as_deref().unwrap_or("anthropic") == "anthropic"
            && config
                .api_key
                .as_ref()
                .is_some_and(|v| !v.trim().is_empty());
        let mut verified = Vec::new();
        for (name, profile) in &settings.models.profiles {
            let success:bool=self.connection()?.query_row("SELECT EXISTS(SELECT 1 FROM model_usage WHERE provider=?1 AND model=?2 AND profile=?3 AND status IN ('completed','usage_unknown'))",params![profile.provider,profile.model,name],|r|r.get(0))?;
            if success && profile.provider == "anthropic" {
                verified.push(name.clone());
            }
        }
        Ok(
            serde_json::json!({"provider":"anthropic","credential_configured":enrolled||environment||configured,"credential_source":if enrolled {"secure_store"} else if environment {"environment"} else if configured {"host_config"} else {"missing"},"profiles":settings.models.profiles,"live_verified":verified.len()==3,"verified_profiles":verified}),
        )
    }
}
