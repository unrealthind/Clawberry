//! Independent, bounded read-only investigations. No external action dispatcher.
use super::{
    models::Profile,
    workers::{ContextItem, WorkerSpec},
    AgentHost, AgentId,
};
use crate::config::Config;
use anyhow::{bail, Result};
use chrono::{Timelike, Utc};
use rusqlite::{params, OptionalExtension, Transaction, TransactionBehavior};
use serde::{Deserialize, Serialize};

pub(super) fn migrate(tx: &Transaction<'_>) -> Result<()> {
    tx.execute_batch("CREATE TABLE IF NOT EXISTS watch_settings(owner_agent TEXT PRIMARY KEY REFERENCES agents(id),settings TEXT NOT NULL,next_due INTEGER NOT NULL DEFAULT 0);
    CREATE TABLE IF NOT EXISTS watch_runs(id TEXT PRIMARY KEY,owner_agent TEXT NOT NULL REFERENCES agents(id),task_id TEXT NOT NULL REFERENCES tasks(id),worker_id TEXT,status TEXT NOT NULL,started_at INTEGER NOT NULL,deadline INTEGER NOT NULL,budget INTEGER NOT NULL,evidence TEXT NOT NULL);
    CREATE TABLE IF NOT EXISTS notifications(id TEXT PRIMARY KEY,owner_agent TEXT NOT NULL REFERENCES agents(id),run_id TEXT NOT NULL UNIQUE REFERENCES watch_runs(id),task_id TEXT NOT NULL,title TEXT NOT NULL,summary TEXT NOT NULL,status TEXT NOT NULL,created_at INTEGER NOT NULL,read INTEGER NOT NULL DEFAULT 0);")?;
    Ok(())
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WatchSettings {
    pub enabled: bool,
    pub interval_minutes: u32,
    /// Explicit UTC hours; the UI labels this rather than guessing DST rules.
    pub quiet_start_utc: u32,
    pub quiet_end_utc: u32,
    pub daily_runs: u32,
    pub daily_budget_microusd: u64,
    pub investigation_budget_microusd: u64,
    pub stale_after_hours: u32,
}
impl Default for WatchSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_minutes: 60,
            quiet_start_utc: 5,
            quiet_end_utc: 14,
            daily_runs: 8,
            daily_budget_microusd: 1_000_000,
            investigation_budget_microusd: 100_000,
            stale_after_hours: 48,
        }
    }
}
impl WatchSettings {
    fn validate(&self) -> Result<()> {
        if !(5..=1440).contains(&self.interval_minutes)
            || self.quiet_start_utc > 23
            || self.quiet_end_utc > 23
            || !(1..=48).contains(&self.daily_runs)
            || self.investigation_budget_microusd == 0
            || self.investigation_budget_microusd > self.daily_budget_microusd
            || self.daily_budget_microusd > 100_000_000
            || !(1..=8760).contains(&self.stale_after_hours)
        {
            bail!("Invalid watch schedule or budget");
        }
        Ok(())
    }
    fn quiet(&self, hour: u32) -> bool {
        if self.quiet_start_utc == self.quiet_end_utc {
            return false;
        }
        if self.quiet_start_utc < self.quiet_end_utc {
            (self.quiet_start_utc..self.quiet_end_utc).contains(&hour)
        } else {
            hour >= self.quiet_start_utc || hour < self.quiet_end_utc
        }
    }
}
type WatchClaim = (String, String, WatchSettings, Vec<ContextItem>);

impl AgentHost {
    pub fn watch_settings(&self, owner: AgentId) -> Result<WatchSettings> {
        let value: Option<String> = self
            .connection()?
            .query_row(
                "SELECT settings FROM watch_settings WHERE owner_agent=?1",
                [owner.as_str()],
                |r| r.get(0),
            )
            .optional()?;
        value.map_or_else(
            || Ok(WatchSettings::default()),
            |s| Ok(serde_json::from_str(&s)?),
        )
    }
    /// Human management API only. Turning a schedule on never grants tools.
    pub fn configure_watch(&self, owner: AgentId, settings: &WatchSettings) -> Result<()> {
        settings.validate()?;
        let mut db = self.connection()?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute("INSERT INTO watch_settings(owner_agent,settings) VALUES(?1,?2) ON CONFLICT(owner_agent) DO UPDATE SET settings=excluded.settings", params![owner.as_str(), serde_json::to_string(settings)?])?;
        tx.execute("INSERT INTO audit_events(request_id,owner_agent,event,detail) VALUES(?1,?2,'watch_configured',?3)", params![uuid::Uuid::new_v4().to_string(),owner.as_str(),serde_json::to_string(settings)?])?;
        tx.commit()?;
        Ok(())
    }
    /// One atomic schedule claim per owner, shared by concurrent daemon hosts.
    fn claim_watch(&self, owner: AgentId) -> Result<Option<WatchClaim>> {
        let mut db = self.connection()?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let stored: Option<(String, i64)> = tx
            .query_row(
                "SELECT settings,next_due FROM watch_settings WHERE owner_agent=?1",
                [owner.as_str()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let Some((json, due)) = stored else {
            return Ok(None);
        };
        let settings: WatchSettings = serde_json::from_str(&json)?;
        settings.validate()?;
        let now = Utc::now();
        if !settings.enabled || due > now.timestamp() || settings.quiet(now.hour()) {
            return Ok(None);
        }
        let pending: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM watch_runs WHERE owner_agent=?1 AND status IN ('preparing','investigating'))",[owner.as_str()], |r|r.get(0))?;
        if pending {
            return Ok(None);
        }
        let day = now
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .expect("midnight")
            .and_utc()
            .timestamp();
        let (count, reserved):(u32,u64) = tx.query_row("SELECT count(*),coalesce(sum(budget),0) FROM watch_runs WHERE owner_agent=?1 AND started_at>=?2",params![owner.as_str(),day],|r|Ok((r.get(0)?,r.get(1)?)))?;
        if count >= settings.daily_runs
            || reserved.saturating_add(settings.investigation_budget_microusd)
                > settings.daily_budget_microusd
        {
            return Ok(None);
        }
        let cutoff = (now - chrono::Duration::hours(i64::from(settings.stale_after_hours)))
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();
        let mut query = tx.prepare("SELECT id,title,substr(description,1,2000),status,due_at,updated_at FROM tasks WHERE owner_agent=?1 AND source<>'heartbeat' AND status NOT IN ('completed','cancelled') AND ((due_at IS NOT NULL AND julianday(due_at)<=julianday('now','+1 day')) OR updated_at<=?2) ORDER BY CASE WHEN due_at IS NULL THEN 1 ELSE 0 END,due_at,updated_at LIMIT 10")?;
        let mut evidence = query.query_map(params![owner.as_str(),cutoff], |r| Ok(ContextItem { source_ref:r.get(0)?,source_type:"task".into(),content:serde_json::json!({"title":r.get::<_,String>(1)?,"description":r.get::<_,String>(2)?,"status":r.get::<_,String>(3)?,"due_at":r.get::<_,Option<String>>(4)?,"updated_at":r.get::<_,String>(5)?}).to_string() }))?.collect::<rusqlite::Result<Vec<_>>>()?;
        drop(query);
        // Include failed read workflows, never their raw payloads or credentials.
        let mut failures = tx.prepare("SELECT request_id,event,detail FROM audit_events WHERE owner_agent=?1 AND event='n8n_read_failed' AND created_at>=datetime('now','-1 day') ORDER BY sequence DESC LIMIT 5")?;
        evidence.extend(failures.query_map([owner.as_str()], |r|Ok(ContextItem {source_ref:r.get(0)?,source_type:"task".into(),content:serde_json::json!({"event":r.get::<_,String>(1)?,"workflow":r.get::<_,String>(2)?}).to_string()}))?.collect::<rusqlite::Result<Vec<_>>>()?);
        drop(failures);
        tx.execute(
            "UPDATE watch_settings SET next_due=?1 WHERE owner_agent=?2",
            params![
                now.timestamp() + i64::from(settings.interval_minutes) * 60,
                owner.as_str()
            ],
        )?;
        if evidence.is_empty() {
            tx.commit()?;
            return Ok(None);
        }
        let run = uuid::Uuid::new_v4().to_string();
        let task = uuid::Uuid::new_v4().to_string();
        tx.execute("INSERT INTO tasks(id,owner_agent,title,description,status,priority,created_by,source) VALUES(?1,?2,'Watch investigation','Review due/stale work and failed workflows; recommend only.','investigating','normal',?2,'heartbeat')",params![task,owner.as_str()])?;
        tx.execute("INSERT INTO task_events(task_id,event,detail) VALUES(?1,'created','Heartbeat read-only investigation')",[&task])?;
        tx.execute("INSERT INTO watch_runs(id,owner_agent,task_id,status,started_at,deadline,budget,evidence) VALUES(?1,?2,?3,'preparing',?4,?5,?6,?7)",params![run,owner.as_str(),task,now.timestamp(),now.timestamp()+120,settings.investigation_budget_microusd,serde_json::to_string(&evidence)?])?;
        tx.execute("INSERT INTO audit_events(request_id,owner_agent,event,detail) VALUES(?1,?2,'watch_investigation_started',?3)",params![run,owner.as_str(),task])?;
        tx.commit()?;
        Ok(Some((run, task, settings, evidence)))
    }
    pub fn watch_tick(&self, config: &Config, owner: AgentId) -> Result<()> {
        self.review_expired_memories(owner)?;
        self.finish_watches(owner)?;
        if self.watch_settings(owner)?.quiet(Utc::now().hour()) {
            return Ok(());
        }
        let Some((run, task, settings, context)) = self.claim_watch(owner)? else {
            return Ok(());
        };
        let result = self.spawn_worker(config, owner, WorkerSpec {task_id:task,role:"watch investigator".into(),objective:"Inspect the supplied owned work evidence. Separate fact, inference and recommendation; identify deadlines and stale commitments. Recommend a concrete next step for User. Do not perform external actions or assert that external state was checked.".into(),context,profile:Profile::Fast,budget_microusd:settings.investigation_budget_microusd,timeout_secs:60,allowed_tools:vec![],allowed_credentials:vec![],retry_deep:false});
        match result {
            Ok(worker) => {
                self.connection()?.execute("UPDATE watch_runs SET worker_id=?1,status='investigating' WHERE id=?2 AND status='preparing'",params![worker.id,run])?;
            }
            Err(_) => {
                self.complete_watch(owner,&run,"failed","Investigation could not start. Check model configuration, concurrency and budgets.")?;
            }
        }
        Ok(())
    }
    fn finish_watches(&self, owner: AgentId) -> Result<()> {
        let rows = self.connection()?.prepare("SELECT id,worker_id,deadline FROM watch_runs WHERE owner_agent=?1 AND status IN ('preparing','investigating')")?.query_map([owner.as_str()],|r|Ok((r.get::<_,String>(0)?,r.get::<_,Option<String>>(1)?,r.get::<_,i64>(2)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
        for (run, worker, deadline) in rows {
            if let Some(worker) = worker {
                let state = self.worker(owner, &worker)?;
                if state.status == "completed" {
                    let summary = state.result.map_or_else(
                        || "Investigation completed; inspect its archive.".into(),
                        |r| r.summary,
                    );
                    self.complete_watch(owner, &run, "completed", &summary)?;
                    continue;
                }
                if ["failed", "cancelled", "timed_out", "interrupted"]
                    .contains(&state.status.as_str())
                {
                    self.complete_watch(owner,&run,"failed","Investigation failed. Retained worker logs explain the failure; no external action was performed.")?;
                    continue;
                }
            }
            if deadline <= Utc::now().timestamp() {
                self.complete_watch(owner,&run,"failed","Investigation outcome needs review after interruption or timeout. It was not automatically replayed.")?;
            }
        }
        Ok(())
    }
    fn complete_watch(&self, owner: AgentId, run: &str, status: &str, summary: &str) -> Result<()> {
        let mut db = self.connection()?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if tx.execute("UPDATE watch_runs SET status=?1 WHERE id=?2 AND owner_agent=?3 AND status IN ('preparing','investigating')",params![status,run,owner.as_str()])? == 0 { return Ok(()); }
        tx.execute("INSERT INTO notifications(id,owner_agent,run_id,task_id,title,summary,status,created_at) SELECT ?1,owner_agent,id,task_id,?2,?3,?4,?5 FROM watch_runs WHERE id=?6",params![uuid::Uuid::new_v4().to_string(),if status=="completed" {"Watch recommendation"} else {"Watch investigation needs attention"},summary,status,Utc::now().timestamp(),run])?;
        tx.execute("UPDATE tasks SET status='needs_manni',updated_at=CURRENT_TIMESTAMP WHERE id=(SELECT task_id FROM watch_runs WHERE id=?1)",[run])?;
        tx.execute("INSERT INTO task_events(task_id,event,detail) SELECT task_id,'watch_finished',?1 FROM watch_runs WHERE id=?2",params![status,run])?;
        tx.execute("INSERT INTO audit_events(request_id,owner_agent,event,detail) VALUES(?1,?2,'watch_investigation_finished',?3)",params![run,owner.as_str(),status])?;
        tx.commit()?;
        Ok(())
    }
    pub fn notifications(
        &self,
        owner: AgentId,
        page: &super::pages::PageRequest,
    ) -> Result<super::pages::Page<serde_json::Value>> {
        let (before, limit) = page.bounds()?;
        let db = self.connection()?;
        let mut q=db.prepare("SELECT n.rowid,n.id,n.task_id,n.title,n.summary,n.status,n.created_at,n.read,w.id,w.worker_id,w.budget,w.evidence FROM notifications n JOIN watch_runs w ON w.id=n.run_id AND w.owner_agent=n.owner_agent WHERE n.owner_agent=?1 AND n.rowid<?2 ORDER BY n.rowid DESC LIMIT ?3")?;
        let rows=q.query_map(params![owner.as_str(),before,limit+1],|r|Ok((r.get::<_,i64>(0)?,serde_json::json!({"id":r.get::<_,String>(1)?,"task_id":r.get::<_,String>(2)?,"title":r.get::<_,String>(3)?,"summary":r.get::<_,String>(4)?,"status":r.get::<_,String>(5)?,"created_at":r.get::<_,i64>(6)?,"read":r.get::<_,bool>(7)?,"run_id":r.get::<_,String>(8)?,"worker_id":r.get::<_,Option<String>>(9)?,"budget_microusd":r.get::<_,u64>(10)?}),r.get::<_,String>(11)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
        let rows = rows
            .into_iter()
            .map(|(cursor, mut value, evidence)| {
                value["evidence"] = serde_json::from_str(&evidence)?;
                Ok((cursor, value))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(super::pages::Page::from_rows(rows, limit))
    }
    pub fn read_notification(&self, owner: AgentId, id: &str) -> Result<()> {
        if self.connection()?.execute(
            "UPDATE notifications SET read=1 WHERE id=?1 AND owner_agent=?2",
            params![id, owner.as_str()],
        )? != 1
        {
            bail!("Notification not found in selected domain");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn enabled() -> WatchSettings {
        WatchSettings {
            enabled: true,
            quiet_start_utc: 0,
            quiet_end_utc: 0,
            ..Default::default()
        }
    }
    fn due(host: &AgentHost, owner: AgentId) {
        let task = host
            .create_task(owner, "Deadline", "Inspect supplied deadline", None)
            .unwrap();
        host.set_task_metadata(owner, &task.id, "high", Some("2026-01-01T00:00:00Z"))
            .unwrap();
    }
    #[test]
    fn defaults_quiet_hours_and_settings_are_independent() {
        let d = tempfile::TempDir::new().unwrap();
        let host = AgentHost::open(d.path()).unwrap();
        assert!(!host.watch_settings(AgentId::Janet).unwrap().enabled);
        let mut settings = enabled();
        settings.quiet_start_utc = 22;
        settings.quiet_end_utc = 7;
        assert!(settings.quiet(23));
        assert!(settings.quiet(0));
        assert!(!settings.quiet(7));
        host.configure_watch(AgentId::Janet, &settings).unwrap();
        assert!(!host.watch_settings(AgentId::Harley).unwrap().enabled);
        settings.interval_minutes = 0;
        assert!(host.configure_watch(AgentId::Janet, &settings).is_err());
    }
    #[test]
    fn expired_investigation_finishes_during_quiet_hours_after_restart() {
        let directory = tempfile::TempDir::new().unwrap();
        let host = AgentHost::open(directory.path()).unwrap();
        due(&host, AgentId::Janet);
        host.configure_watch(AgentId::Janet, &enabled()).unwrap();
        let (run, _, _, _) = host.claim_watch(AgentId::Janet).unwrap().unwrap();
        host.connection()
            .unwrap()
            .execute("UPDATE watch_runs SET deadline=0 WHERE id=?1", [&run])
            .unwrap();
        let hour = Utc::now().hour();
        host.configure_watch(
            AgentId::Janet,
            &WatchSettings {
                enabled: false,
                quiet_start_utc: hour,
                quiet_end_utc: (hour + 1) % 24,
                ..enabled()
            },
        )
        .unwrap();
        drop(host);
        let host = AgentHost::open(directory.path()).unwrap();
        host.watch_tick(&Config::default(), AgentId::Janet).unwrap();
        host.watch_tick(&Config::default(), AgentId::Janet).unwrap();
        let notifications = host
            .notifications(AgentId::Janet, &super::super::pages::PageRequest::default())
            .unwrap();
        assert_eq!(notifications.items.len(), 1);
        assert_eq!(notifications.items[0]["status"], "failed");
        assert_eq!(notifications.items[0]["run_id"], run);
        assert_eq!(notifications.items[0]["evidence"][0]["source_type"], "task");
        assert!(notifications.items[0]["worker_id"].is_null());
        assert!(host.workers(AgentId::Janet).unwrap().is_empty());
        assert!(host
            .notifications(
                AgentId::Harley,
                &super::super::pages::PageRequest::default()
            )
            .unwrap()
            .items
            .is_empty());
    }

    #[test]
    fn atomic_claim_deduplicates_and_preserves_domain_and_daily_budget() {
        let d = tempfile::TempDir::new().unwrap();
        let host = AgentHost::open(d.path()).unwrap();
        due(&host, AgentId::Janet);
        due(&host, AgentId::Harley);
        let settings = WatchSettings {
            daily_runs: 1,
            ..enabled()
        };
        host.configure_watch(AgentId::Janet, &settings).unwrap();
        let other = AgentHost::open(d.path()).unwrap();
        let (run, task, _, context) = host.claim_watch(AgentId::Janet).unwrap().unwrap();
        assert_eq!(context.len(), 1);
        assert_eq!(
            host.task(AgentId::Janet, &context[0].source_ref)
                .unwrap()
                .owner_agent,
            AgentId::Janet
        );
        assert!(other.claim_watch(AgentId::Janet).unwrap().is_none());
        assert!(other.claim_watch(AgentId::Harley).unwrap().is_none());
        host.complete_watch(AgentId::Janet, &run, "failed", "Failure retained")
            .unwrap();
        host.complete_watch(AgentId::Janet, &run, "failed", "Duplicate")
            .unwrap();
        assert_eq!(
            host.notifications(AgentId::Janet, &super::super::pages::PageRequest::default())
                .unwrap()
                .items
                .len(),
            1
        );
        assert_eq!(
            host.notifications(
                AgentId::Harley,
                &super::super::pages::PageRequest::default()
            )
            .unwrap()
            .items
            .len(),
            0
        );
        assert_eq!(
            host.task(AgentId::Janet, &task).unwrap().status,
            super::super::work::TaskStatus::NeedsManni
        );
        host.connection()
            .unwrap()
            .execute("UPDATE watch_settings SET next_due=0", [])
            .unwrap();
        assert!(other.claim_watch(AgentId::Janet).unwrap().is_none());
    }
    #[tokio::test]
    async fn failed_start_notifies_without_tools_and_does_not_repeat() {
        let d = tempfile::TempDir::new().unwrap();
        let host = AgentHost::open(d.path()).unwrap();
        due(&host, AgentId::Janet);
        host.configure_watch(AgentId::Janet, &enabled()).unwrap();
        let config = Config {
            default_provider: Some("invalid-watch-test-provider".into()),
            ..Config::default()
        };
        host.watch_tick(&config, AgentId::Janet).unwrap();
        // A worker can start and then fail without model configuration. Polling
        // retains the archive and emits at most one notification for the run.
        host.wait_active_workers(AgentId::Janet).await.unwrap();
        host.finish_watches(AgentId::Janet).unwrap();
        let rows = host
            .notifications(AgentId::Janet, &super::super::pages::PageRequest::default())
            .unwrap();
        assert_eq!(rows.items.len(), 1);
        let workers = host.workers(AgentId::Janet).unwrap();
        assert!(workers
            .iter()
            .all(|w| w.spec.allowed_tools.is_empty() && w.spec.allowed_credentials.is_empty()));
        let id = rows.items[0]["id"].as_str().unwrap();
        assert!(host.read_notification(AgentId::Harley, id).is_err());
        host.read_notification(AgentId::Janet, id).unwrap();
    }
}
