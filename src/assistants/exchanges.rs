//! Explicitly reviewed, single-exchange peer messages. Never peer DB access.
//! Both question and answer are immutable and hash-bound to human review.
use super::{AgentHost, AgentId};
use anyhow::{bail, Result};
use rusqlite::{params, Transaction, TransactionBehavior};
use serde::Serialize;
use sha2::{Digest, Sha256};

pub(super) fn migrate(tx: &Transaction<'_>) -> Result<()> {
    tx.execute_batch("CREATE TABLE IF NOT EXISTS peer_exchanges (
        id TEXT PRIMARY KEY,requester TEXT NOT NULL REFERENCES agents(id),recipient TEXT NOT NULL REFERENCES agents(id),
        task_id TEXT NOT NULL REFERENCES tasks(id),question TEXT NOT NULL,question_hash TEXT NOT NULL,
        response TEXT,response_hash TEXT,status TEXT NOT NULL DEFAULT 'pending',expires_at INTEGER NOT NULL,
        CHECK(requester<>recipient));")?;
    Ok(())
}
#[derive(Debug, Serialize)]
pub struct Exchange {
    pub id: String,
    pub requester: AgentId,
    pub recipient: AgentId,
    pub task_id: String,
    pub question: String,
    pub question_hash: String,
    pub response: Option<String>,
    pub response_hash: Option<String>,
    pub status: String,
    pub expires_at: i64,
}
fn hash(kind: &str, content: &str, exchange: &Exchange) -> String {
    hex::encode(Sha256::digest(serde_json::json!({"kind":kind,"id":exchange.id,"expires_at":exchange.expires_at,"requester":exchange.requester,"recipient":exchange.recipient,"task_id":exchange.task_id,"content":content}).to_string()))
}
impl AgentHost {
    pub fn request_exchange(
        &self,
        owner: AgentId,
        recipient: AgentId,
        task: &str,
        question: &str,
    ) -> Result<String> {
        if owner == recipient {
            bail!("Peer exchange requires two different permanent agents");
        }
        self.task(owner, task)?;
        super::work::text(question, 4_000, "Exact cross-boundary question")?;
        let id = uuid::Uuid::new_v4().to_string();
        let exchange = Exchange {
            id: id.clone(),
            requester: owner,
            recipient,
            task_id: task.into(),
            question: question.into(),
            question_hash: String::new(),
            response: None,
            response_hash: None,
            status: "pending".into(),
            expires_at: chrono::Utc::now().timestamp() + 900,
        };
        let mut db = self.connection()?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute("INSERT INTO peer_exchanges(id,requester,recipient,task_id,question,question_hash,expires_at) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![id,owner.as_str(),recipient.as_str(),task,question,hash("question",question,&exchange),exchange.expires_at])?;
        tx.execute("INSERT INTO audit_events(request_id,owner_agent,event,detail) VALUES(?1,?2,'peer_request_created',?3)",params![id,owner.as_str(),recipient.as_str()])?;
        tx.commit()?;
        Ok(id)
    }

    fn expire_exchanges(&self) -> Result<()> {
        let mut db = self.connection()?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute("INSERT INTO audit_events(request_id,owner_agent,event,detail) SELECT id,requester,'peer_exchange_expired','Scope closed without new delivery' FROM peer_exchanges WHERE expires_at<=?1 AND status NOT IN ('closed','revoked','expired')",[chrono::Utc::now().timestamp()])?;
        tx.execute("UPDATE peer_exchanges SET status='expired' WHERE expires_at<=?1 AND status NOT IN ('closed','revoked','expired')",[chrono::Utc::now().timestamp()])?;
        tx.commit()?;
        Ok(())
    }

    /// Human review may inspect both pending payloads. The model path cannot.
    pub fn exchanges(&self, owner: AgentId, human_review: bool) -> Result<Vec<Exchange>> {
        self.expire_exchanges()?;
        let db = self.connection()?;
        let mut q=db.prepare("SELECT id,requester,recipient,task_id,question,question_hash,response,response_hash,status,expires_at FROM peer_exchanges WHERE (requester=?1 OR (recipient=?1 AND status IN ('approved','response_pending','closed'))) ORDER BY rowid DESC LIMIT 100")?;
        let rows = q.query_map([owner.as_str()], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, Option<String>>(6)?,
                r.get::<_, Option<String>>(7)?,
                r.get::<_, String>(8)?,
                r.get::<_, i64>(9)?,
            ))
        })?;
        rows.map(|row| {
            let r = row?;
            let requester = AgentId::parse(&r.1)?;
            let recipient = AgentId::parse(&r.2)?;
            let reveal = human_review || r.8 == "closed" || recipient == owner;
            Ok(Exchange {
                id: r.0,
                requester,
                recipient,
                task_id: r.3,
                question: r.4,
                question_hash: r.5,
                response: if reveal { r.6 } else { None },
                response_hash: if reveal { r.7 } else { None },
                status: r.8,
                expires_at: r.9,
            })
        })
        .collect()
    }

    /// Dedicated authenticated human path. Review must bind the exact displayed hash.
    pub fn review_exchange(
        &self,
        owner: AgentId,
        id: &str,
        reviewed_hash: &str,
        decision: &str,
    ) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        let mut db = self.connection()?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (recipient, task, question, response, expires_at): (String, String, String, Option<String>, i64) = tx.query_row(
            "SELECT recipient,task_id,question,response,expires_at FROM peer_exchanges WHERE id=?1 AND requester=?2",
            params![id,owner.as_str()], |row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?)),
        )?;
        let exchange = Exchange {
            id: id.into(),
            requester: owner,
            recipient: AgentId::parse(&recipient)?,
            task_id: task,
            question,
            question_hash: String::new(),
            response,
            response_hash: None,
            status: String::new(),
            expires_at,
        };
        let expected = if decision == "approve_response" {
            hash(
                "response",
                exchange.response.as_deref().unwrap_or(""),
                &exchange,
            )
        } else {
            hash("question", &exchange.question, &exchange)
        };
        if decision != "revoke" && expected != reviewed_hash {
            bail!("Exchange review must bind its current identity, expiry and exact payload; recreate legacy requests");
        }
        let (sql,event)=match decision {
            "approve_question"=>("UPDATE peer_exchanges SET status='approved' WHERE id=?1 AND requester=?2 AND question_hash=?3 AND status='pending' AND expires_at>?4","peer_question_approved"),
            "approve_response"=>("UPDATE peer_exchanges SET status='closed' WHERE id=?1 AND requester=?2 AND response_hash=?3 AND status='response_pending' AND expires_at>?4","peer_response_delivered"),
            "revoke"=>("UPDATE peer_exchanges SET status='revoked' WHERE id=?1 AND requester=?2 AND question_hash=?3 AND status NOT IN ('closed','expired','revoked') AND expires_at>?4","peer_exchange_revoked"),
            _=>bail!("Unknown peer review decision"),
        };
        if tx.execute(sql, params![id, owner.as_str(), reviewed_hash, now])? != 1 {
            bail!("Exchange scope, payload hash, state, or expiry mismatch");
        }
        tx.execute(
            "INSERT INTO audit_events(request_id,owner_agent,event,detail) VALUES(?1,?2,?3,?4)",
            params![id, owner.as_str(), event, reviewed_hash],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn respond_exchange(&self, owner: AgentId, id: &str, response: &str) -> Result<()> {
        super::work::text(response, 4_000, "Scoped peer response")?;
        let mut db = self.connection()?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (requester,task,expires_at,question,question_hash):(String,String,i64,String,String)=tx.query_row("SELECT requester,task_id,expires_at,question,question_hash FROM peer_exchanges WHERE id=?1 AND recipient=?2 AND status='approved' AND expires_at>?3",params![id,owner.as_str(),chrono::Utc::now().timestamp()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?)))?;
        let exchange = Exchange {
            id: id.into(),
            requester: AgentId::parse(&requester)?,
            recipient: owner,
            task_id: task,
            question,
            question_hash,
            response: None,
            response_hash: None,
            status: "approved".into(),
            expires_at,
        };
        if hash("question", &exchange.question, &exchange) != exchange.question_hash {
            bail!("Legacy or changed exchange scope requires a fresh request and human review");
        }
        let response_hash = hash("response", response, &exchange);
        if tx.execute("UPDATE peer_exchanges SET response=?1,response_hash=?2,status='response_pending' WHERE id=?3 AND recipient=?4 AND status='approved'",params![response,response_hash,id,owner.as_str()])?!=1 { bail!("Exchange was already consumed"); }
        tx.execute("INSERT INTO audit_events(request_id,owner_agent,event,detail) VALUES(?1,?2,'peer_response_prepared',?3)",params![id,owner.as_str(),response_hash])?;
        tx.commit()?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn question_and_response_review_hashes_are_domain_separated() {
        let exchange = Exchange {
            id: "exchange".into(),
            requester: AgentId::Janet,
            recipient: AgentId::Harley,
            task_id: "task".into(),
            question: String::new(),
            question_hash: String::new(),
            response: None,
            response_hash: None,
            status: "pending".into(),
            expires_at: 100,
        };
        assert_ne!(
            hash("question", "same text", &exchange),
            hash("response", "same text", &exchange)
        );
    }
    #[test]
    fn peer_exchange_is_reviewed_exact_single_use_and_closed() {
        let d = tempfile::TempDir::new().unwrap();
        let h = AgentHost::open(d.path()).unwrap();
        let task = h
            .create_task(
                AgentId::Janet,
                "Coordinate",
                "Ask about a shared appointment",
                None,
            )
            .unwrap();
        let id = h
            .request_exchange(
                AgentId::Janet,
                AgentId::Harley,
                &task.id,
                "Which hour was approved?",
            )
            .unwrap();
        assert!(h.exchanges(AgentId::Harley, false).unwrap().is_empty());
        assert!(h.respond_exchange(AgentId::Harley, &id, "10 AM").is_err());
        let q = h.exchanges(AgentId::Janet, true).unwrap().remove(0);
        assert!(h
            .review_exchange(AgentId::Janet, &id, "wrong hash", "approve_question")
            .is_err());
        h.review_exchange(AgentId::Janet, &id, &q.question_hash, "approve_question")
            .unwrap();
        h.respond_exchange(AgentId::Harley, &id, "10 AM").unwrap();
        assert!(h.respond_exchange(AgentId::Harley, &id, "11 AM").is_err());
        assert!(h.exchanges(AgentId::Janet, false).unwrap()[0]
            .response
            .is_none());
        let response = h.exchanges(AgentId::Janet, true).unwrap().remove(0);
        h.review_exchange(
            AgentId::Janet,
            &id,
            response.response_hash.as_deref().unwrap(),
            "approve_response",
        )
        .unwrap();
        assert_eq!(
            h.exchanges(AgentId::Janet, false).unwrap()[0]
                .response
                .as_deref(),
            Some("10 AM")
        );
        assert!(h
            .review_exchange(
                AgentId::Janet,
                &id,
                response.response_hash.as_deref().unwrap(),
                "approve_response"
            )
            .is_err());
    }
    #[test]
    fn identical_questions_have_distinct_review_grants() {
        let directory = tempfile::TempDir::new().unwrap();
        let host = AgentHost::open(directory.path()).unwrap();
        let task = host
            .create_task(AgentId::Janet, "Coordinate", "Question", None)
            .unwrap();
        let first = host
            .request_exchange(AgentId::Janet, AgentId::Harley, &task.id, "Same question")
            .unwrap();
        let second = host
            .request_exchange(AgentId::Janet, AgentId::Harley, &task.id, "Same question")
            .unwrap();
        let requests = host.exchanges(AgentId::Janet, true).unwrap();
        let first_hash = &requests
            .iter()
            .find(|request| request.id == first)
            .unwrap()
            .question_hash;
        let second_hash = &requests
            .iter()
            .find(|request| request.id == second)
            .unwrap()
            .question_hash;
        assert_ne!(first_hash, second_hash);
        assert!(host
            .review_exchange(AgentId::Janet, &second, first_hash, "approve_question")
            .is_err());
        host.review_exchange(AgentId::Janet, &first, first_hash, "approve_question")
            .unwrap();
        drop(host);
        let host = AgentHost::open(directory.path()).unwrap();
        host.respond_exchange(AgentId::Harley, &first, "Scoped answer")
            .unwrap();
        assert!(host
            .respond_exchange(AgentId::Harley, &second, "Unapproved")
            .is_err());
        assert!(host
            .exchanges(AgentId::Janet, false)
            .unwrap()
            .iter()
            .all(|request| request.response.is_none()));
    }

    #[test]
    fn legacy_hashes_and_changed_expiry_cannot_authorize_delivery() {
        let directory = tempfile::TempDir::new().unwrap();
        let host = AgentHost::open(directory.path()).unwrap();
        let task = host
            .create_task(AgentId::Janet, "Coordinate", "Question", None)
            .unwrap();
        let id = host
            .request_exchange(AgentId::Janet, AgentId::Harley, &task.id, "Question")
            .unwrap();
        let reviewed = host.exchanges(AgentId::Janet, true).unwrap().remove(0);
        host.connection()
            .unwrap()
            .execute(
                "UPDATE peer_exchanges SET expires_at=expires_at+60 WHERE id=?1",
                [&id],
            )
            .unwrap();
        assert!(host
            .review_exchange(
                AgentId::Janet,
                &id,
                &reviewed.question_hash,
                "approve_question"
            )
            .is_err());
        host.connection()
            .unwrap()
            .execute(
                "UPDATE peer_exchanges SET question_hash='legacy' WHERE id=?1",
                [&id],
            )
            .unwrap();
        assert!(host
            .review_exchange(AgentId::Janet, &id, "legacy", "approve_question")
            .is_err());
        host.connection()
            .unwrap()
            .execute(
                "UPDATE peer_exchanges SET status='approved' WHERE id=?1",
                [&id],
            )
            .unwrap();
        assert!(host
            .respond_exchange(AgentId::Harley, &id, "Not delivered")
            .is_err());
        host.review_exchange(AgentId::Janet, &id, "legacy", "revoke")
            .unwrap();
        assert_eq!(
            host.exchanges(AgentId::Janet, true).unwrap()[0].status,
            "revoked"
        );
    }

    #[test]
    fn revocation_expiry_and_wrong_owner_prevent_delivery() {
        let d = tempfile::TempDir::new().unwrap();
        let h = AgentHost::open(d.path()).unwrap();
        let task = h
            .create_task(AgentId::Janet, "Coordinate", "Question", None)
            .unwrap();
        let id = h
            .request_exchange(AgentId::Janet, AgentId::Harley, &task.id, "Question")
            .unwrap();
        let q = h.exchanges(AgentId::Janet, true).unwrap().remove(0);
        assert!(h
            .review_exchange(AgentId::Harley, &id, &q.question_hash, "approve_question")
            .is_err());
        h.review_exchange(AgentId::Janet, &id, &q.question_hash, "revoke")
            .unwrap();
        assert!(h.respond_exchange(AgentId::Harley, &id, "Answer").is_err());
        let id = h
            .request_exchange(AgentId::Janet, AgentId::Harley, &task.id, "Question")
            .unwrap();
        let q = h.exchanges(AgentId::Janet, true).unwrap().remove(0);
        h.connection()
            .unwrap()
            .execute("UPDATE peer_exchanges SET expires_at=0 WHERE id=?1", [&id])
            .unwrap();
        assert!(h
            .review_exchange(AgentId::Janet, &id, &q.question_hash, "approve_question")
            .is_err());
        assert!(h.respond_exchange(AgentId::Harley, &id, "Answer").is_err());
    }
}
