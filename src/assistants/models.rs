//! Provider-independent profiles, integer cost accounting, and atomic budgets.
use super::{AgentHost, AgentId};
use crate::config::Config;
use crate::providers::{self, ChatMessage, UsageTracker};
use anyhow::{bail, Context, Result};
use rusqlite::{params, Transaction};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::Duration;
use uuid::Uuid;

pub(super) fn migrate(tx: &Transaction<'_>) -> Result<()> {
    tx.execute_batch("CREATE TABLE IF NOT EXISTS model_usage (
        id TEXT PRIMARY KEY, owner_agent TEXT NOT NULL REFERENCES agents(id), task_id TEXT, worker_id TEXT,
        profile TEXT NOT NULL, provider TEXT NOT NULL, model TEXT NOT NULL,
        input_tokens INTEGER NOT NULL DEFAULT 0, output_tokens INTEGER NOT NULL DEFAULT 0,
        cost_microusd INTEGER NOT NULL, reserved_microusd INTEGER NOT NULL,
        status TEXT NOT NULL, usage_reported INTEGER NOT NULL DEFAULT 0,
        created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
    ); CREATE INDEX IF NOT EXISTS usage_owner_day ON model_usage(owner_agent,created_at);
    CREATE INDEX IF NOT EXISTS usage_task ON model_usage(task_id);")?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Profile {
    Fast,
    Normal,
    Deep,
}
impl Profile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::Normal => "normal",
            Self::Deep => "deep",
        }
    }
    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "fast" => Ok(Self::Fast),
            "normal" => Ok(Self::Normal),
            "deep" => Ok(Self::Deep),
            _ => bail!("Unknown reasoning profile"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelProfile {
    pub provider: String,
    pub model: String,
    pub input_microusd_per_million: u64,
    pub output_microusd_per_million: u64,
    #[serde(default = "output_limit")]
    pub max_output_tokens: u32,
}
fn output_limit() -> u32 {
    4096
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelSettings {
    pub profiles: BTreeMap<String, ModelProfile>,
    #[serde(default = "daily_limit")]
    pub agent_daily_microusd: u64,
    #[serde(default = "deep_limit")]
    pub deep_daily_microusd: u64,
    #[serde(default = "task_limit")]
    pub task_microusd: u64,
}
fn daily_limit() -> u64 {
    5_000_000
}
fn deep_limit() -> u64 {
    2_000_000
}
fn task_limit() -> u64 {
    2_000_000
}

impl ModelSettings {
    pub fn load(config: &Config) -> Result<Self> {
        let path = config
            .config_path
            .parent()
            .context("Config has no directory")?
            .join("model-profiles.toml");
        let result = if path.exists() {
            super::reject_link(&path)?;
            toml::from_str::<Self>(&std::fs::read_to_string(path)?)?
        } else {
            let mut profiles = BTreeMap::new();
            // Existing configuration remains the administrator's normal-profile override.
            for (profile, model, input, output) in [
                ("fast", "claude-haiku-4-5-20251001", 1_000_000, 5_000_000),
                ("normal", "claude-sonnet-5-5", 2_000_000, 10_000_000),
                ("deep", "claude-opus-5-5", 4_000_000, 20_000_000),
            ] {
                profiles.insert(
                    profile.into(),
                    ModelProfile {
                        provider: "anthropic".into(),
                        model: model.into(),
                        input_microusd_per_million: input,
                        output_microusd_per_million: output,
                        max_output_tokens: output_limit(),
                    },
                );
            }
            profiles.insert(
                "normal".into(),
                ModelProfile {
                    provider: config
                        .default_provider
                        .clone()
                        .unwrap_or_else(|| "anthropic".into()),
                    model: config
                        .default_model
                        .clone()
                        .unwrap_or_else(|| "claude-sonnet-5-5".into()),
                    input_microusd_per_million: 2_000_000,
                    output_microusd_per_million: 10_000_000,
                    max_output_tokens: output_limit(),
                },
            );
            for route in &config.model_routes {
                if let Some(entry) = profiles.get_mut(&route.hint) {
                    entry.provider.clone_from(&route.provider);
                    entry.model.clone_from(&route.model);
                }
            }
            Self {
                profiles,
                agent_daily_microusd: daily_limit(),
                deep_daily_microusd: deep_limit(),
                task_microusd: task_limit(),
            }
        };
        result.validate()?;
        Ok(result)
    }
    pub fn validate(&self) -> Result<()> {
        for (name, profile) in &self.profiles {
            Profile::parse(name)?;
            if profile.provider.trim().is_empty()
                || profile.model.trim().is_empty()
                || profile.max_output_tokens == 0
                || profile.max_output_tokens > 32_000
            {
                bail!("Invalid model profile {name}");
            }
            cost(1_000_000, 1_000_000, profile)?;
        }
        for required in ["fast", "normal", "deep"] {
            if !self.profiles.contains_key(required) {
                bail!("Missing {required} model profile; no silent fallback is allowed");
            }
        }
        Ok(())
    }
}

pub fn cost(input: u64, output: u64, profile: &ModelProfile) -> Result<u64> {
    let amount = (u128::from(input) * u128::from(profile.input_microusd_per_million))
        .checked_add(u128::from(output) * u128::from(profile.output_microusd_per_million))
        .context("Model cost overflow")?;
    let value = u64::try_from(amount.div_ceil(1_000_000)).context("Model cost overflow")?;
    i64::try_from(value).context("Model cost exceeds storage range")?;
    Ok(value)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecord {
    pub id: String,
    pub owner_agent: AgentId,
    pub task_id: Option<String>,
    pub worker_id: Option<String>,
    pub profile: String,
    pub provider: String,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_microusd: u64,
    pub status: String,
    pub usage_reported: bool,
}

#[derive(Debug, Clone)]
pub struct InferenceScope<'a> {
    pub owner: AgentId,
    pub task: Option<&'a str>,
    pub worker: Option<&'a str>,
    pub profile: Profile,
    pub budget_microusd: Option<u64>,
}

impl AgentHost {
    fn reserve_inference(
        &self,
        settings: &ModelSettings,
        model: &ModelProfile,
        scope: &InferenceScope<'_>,
        reserve: u64,
    ) -> Result<String> {
        if let Some(task) = scope.task {
            self.task(scope.owner, task)?;
        }
        if let Some(id) = scope.worker {
            let worker = self.worker(scope.owner, id)?;
            if scope.task != Some(worker.task_id.as_str()) || !worker.active {
                bail!("Inference must belong to an active worker and its owned task");
            }
            if scope.budget_microusd != Some(worker.spec.budget_microusd) {
                bail!("Worker inference budget must match its granted budget");
            }
        }
        let mut db = self.connection()?;
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let daily:u64=tx.query_row("SELECT COALESCE(SUM(cost_microusd),0) FROM model_usage WHERE owner_agent=?1 AND date(created_at)=date('now')",[scope.owner.as_str()],|r|r.get(0))?;
        if daily.saturating_add(reserve) > settings.agent_daily_microusd {
            bail!("Agent daily budget exhausted; reasoning was not downgraded");
        }
        if scope.profile == Profile::Deep {
            let spent:u64=tx.query_row("SELECT COALESCE(SUM(cost_microusd),0) FROM model_usage WHERE owner_agent=?1 AND profile='deep' AND date(created_at)=date('now')",[scope.owner.as_str()],|r|r.get(0))?;
            if spent.saturating_add(reserve) > settings.deep_daily_microusd {
                bail!("Deep reasoning budget exhausted; User must review the budget");
            }
        }
        if let Some(task) = scope.task {
            let spent:u64=tx.query_row("SELECT COALESCE(SUM(cost_microusd),0) FROM model_usage WHERE owner_agent=?1 AND task_id=?2",params![scope.owner.as_str(),task],|r|r.get(0))?;
            if spent.saturating_add(reserve) > settings.task_microusd {
                bail!("Task model budget exhausted");
            }
        }
        if let (Some(worker), Some(limit)) = (scope.worker, scope.budget_microusd) {
            let spent:u64=tx.query_row("SELECT COALESCE(SUM(cost_microusd),0) FROM model_usage WHERE owner_agent=?1 AND worker_id=?2",params![scope.owner.as_str(),worker],|r|r.get(0))?;
            if spent.saturating_add(reserve) > limit {
                bail!("Worker model budget exhausted");
            }
        }
        let id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO model_usage(id,owner_agent,task_id,worker_id,profile,provider,model,cost_microusd,reserved_microusd,status) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?8,'reserved')",params![id,scope.owner.as_str(),scope.task,scope.worker,scope.profile.as_str(),model.provider,model.model,i64::try_from(reserve)?])?;
        tx.commit()?;
        Ok(id)
    }

    pub async fn infer(
        &self,
        config: &Config,
        scope: InferenceScope<'_>,
        history: &[ChatMessage],
    ) -> Result<String> {
        self.infer_stream(config, scope, history, None).await
    }

    pub async fn infer_stream(
        &self,
        config: &Config,
        scope: InferenceScope<'_>,
        history: &[ChatMessage],
        token_tx: Option<tokio::sync::mpsc::Sender<String>>,
    ) -> Result<String> {
        let settings = self.global_settings(config)?.value.models;
        let model = settings
            .profiles
            .get(scope.profile.as_str())
            .context("Profile not configured")?;
        let enrolled = if model.provider == "anthropic" {
            crate::security::secrets::claude_credential(config)?
        } else {
            None
        };
        let key = enrolled.as_deref().or_else(|| {
            config
                .model_routes
                .iter()
                .find(|r| r.hint == scope.profile.as_str() && r.provider == model.provider)
                .and_then(|r| r.api_key.as_deref())
                .or_else(|| {
                    if config.default_provider.as_deref().unwrap_or("anthropic") == model.provider {
                        config.api_key.as_deref()
                    } else {
                        None
                    }
                })
        });
        // One exact provider per recorded attempt: retries/escalations are explicit and separately metered.
        let mut provider = providers::create_provider(&model.provider, key)?;
        provider.set_output_token_limit(model.max_output_tokens)?;
        let tracker = UsageTracker::new();
        provider.set_usage_tracker(tracker.clone());
        let input_bound = history
            .iter()
            .try_fold(1024_u64, |sum, m| {
                sum.checked_add(u64::try_from(m.content.len() + m.role.len() + 32).ok()?)
            })
            .context("Context size overflow")?;
        if input_bound > 256_000 {
            bail!("Model context exceeds the local input bound");
        }
        let reserve = cost(input_bound, u64::from(model.max_output_tokens), model)?;
        let id = self.reserve_inference(&settings, model, &scope, reserve)?;
        self.audit(&id, scope.owner, "model_request", scope.profile.as_str())?;
        let mut lease = InferenceLease {
            host: self.clone(),
            id: id.clone(),
            complete: false,
        };
        let result = tokio::time::timeout(Duration::from_secs(25), async {
            if let Some(tx) = token_tx {
                provider
                    .chat_with_history_stream(history, &model.model, config.default_temperature, tx)
                    .await
            } else {
                provider
                    .chat_with_history(history, &model.model, config.default_temperature)
                    .await
            }
        })
        .await;
        let usage = tracker.snapshot();
        let reported = tracker.requests() > 0;
        let actual = if reported {
            cost(usage.prompt_tokens, usage.completion_tokens, model)?
        } else {
            reserve
        };
        let status = match &result {
            Ok(Ok(_)) if reported => "completed",
            Ok(Ok(_)) => "usage_unknown",
            Ok(Err(_)) => "failed_uncertain",
            Err(_) => "timeout_uncertain",
        };
        self.connection()?.execute("UPDATE model_usage SET input_tokens=?1,output_tokens=?2,cost_microusd=?3,status=?4,usage_reported=?5 WHERE id=?6",params![i64::try_from(usage.prompt_tokens)?,i64::try_from(usage.completion_tokens)?,i64::try_from(actual)?,status,reported,id])?;
        lease.complete = true;
        if actual > reserve {
            self.audit(
                &id,
                scope.owner,
                "budget_overrun",
                "Provider usage exceeded reserved bound; charged actual cost",
            )?;
            bail!("Provider exceeded its token/cost bound; inspect usage before retrying");
        }
        result
            .context("Model request timed out; reservation retained when usage is uncertain")?
            .context("Model request failed")
    }

    pub fn usage(&self, owner: AgentId) -> Result<Vec<UsageRecord>> {
        let db = self.connection()?;
        let mut q=db.prepare("SELECT id,task_id,worker_id,profile,provider,model,input_tokens,output_tokens,cost_microusd,status,usage_reported FROM model_usage WHERE owner_agent=?1 ORDER BY created_at,id")?;
        let rows = q.query_map([owner.as_str()], |r| {
            Ok(UsageRecord {
                id: r.get(0)?,
                owner_agent: owner,
                task_id: r.get(1)?,
                worker_id: r.get(2)?,
                profile: r.get(3)?,
                provider: r.get(4)?,
                model: r.get(5)?,
                input_tokens: r.get(6)?,
                output_tokens: r.get(7)?,
                cost_microusd: r.get(8)?,
                status: r.get(9)?,
                usage_reported: r.get(10)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }
}

// Cancelling an HTTP stream drops the inference future. Keep its reservation,
// but make that uncertainty inspectable rather than leaving a running record.
struct InferenceLease {
    host: AgentHost,
    id: String,
    complete: bool,
}
impl Drop for InferenceLease {
    fn drop(&mut self) {
        if !self.complete {
            if let Ok(db) = self.host.connection() {
                if let Err(error) = db.execute("UPDATE model_usage SET status='cancelled_uncertain' WHERE id=?1 AND status='reserved'", [&self.id]) {
                    tracing::error!("Could not record interrupted inference: {error}");
                }
            }
        }
    }
}

#[cfg(test)]
mod budget_tests {
    use super::*;
    use std::sync::{Arc, Barrier};

    fn settings() -> ModelSettings {
        ModelSettings {
            profiles: BTreeMap::new(),
            agent_daily_microusd: 3,
            deep_daily_microusd: 1,
            task_microusd: 3,
        }
    }
    fn model() -> ModelProfile {
        ModelProfile {
            provider: "fixture".into(),
            model: "fixture".into(),
            input_microusd_per_million: 1_000_000,
            output_microusd_per_million: 2_000_000,
            max_output_tokens: 10,
        }
    }

    #[test]
    fn reservations_are_atomic_across_independent_hosts() {
        let dir = tempfile::TempDir::new().unwrap();
        let hosts = [
            AgentHost::open(dir.path()).unwrap(),
            AgentHost::open(dir.path()).unwrap(),
        ];
        let barrier = Arc::new(Barrier::new(2));
        let handles = hosts
            .into_iter()
            .map(|host| {
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    host.reserve_inference(
                        &settings(),
                        &model(),
                        &InferenceScope {
                            owner: AgentId::Janet,
                            task: None,
                            worker: None,
                            profile: Profile::Normal,
                            budget_microusd: None,
                        },
                        2,
                    )
                    .is_ok()
                })
            })
            .collect::<Vec<_>>();
        let succeeded = handles
            .into_iter()
            .map(|handle| usize::from(handle.join().unwrap()))
            .sum::<usize>();
        assert_eq!(succeeded, 1);
        let host = AgentHost::open(dir.path()).unwrap();
        assert_eq!(host.usage(AgentId::Janet).unwrap().len(), 1);
    }

    #[test]
    fn deep_refusal_and_owned_task_scope_do_not_create_usage() {
        let dir = tempfile::TempDir::new().unwrap();
        let host = AgentHost::open(dir.path()).unwrap();
        let scope = InferenceScope {
            owner: AgentId::Janet,
            task: None,
            worker: None,
            profile: Profile::Deep,
            budget_microusd: None,
        };
        assert!(host
            .reserve_inference(&settings(), &model(), &scope, 2)
            .is_err());
        let personal = host
            .create_task(AgentId::Harley, "Private", "Private", None)
            .unwrap();
        assert!(host
            .reserve_inference(
                &settings(),
                &model(),
                &InferenceScope {
                    task: Some(&personal.id),
                    profile: Profile::Normal,
                    ..scope
                },
                1
            )
            .is_err());
        assert!(host.usage(AgentId::Janet).unwrap().is_empty());
    }

    #[test]
    fn integer_prices_round_up_and_reject_overflow() {
        assert_eq!(cost(3, 2, &model()).unwrap(), 7);
        let mut fractional = model();
        fractional.input_microusd_per_million = 1;
        assert_eq!(cost(1, 0, &fractional).unwrap(), 1);
        fractional.input_microusd_per_million = u64::MAX;
        assert!(cost(u64::MAX, 0, &fractional).is_err());
    }
}
