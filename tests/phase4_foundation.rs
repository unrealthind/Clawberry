use serde_json::{json, Value};
use tempfile::TempDir;
use tinyclaw::assistants::{AgentHost, AgentId};
use tinyclaw::Config;

async fn operate(host: &AgentHost, owner: AgentId, payload: Value) -> Value {
    host.operate(&Config::default(), owner, payload)
        .await
        .unwrap()
}

#[tokio::test]
async fn shared_memory_management_contract_survives_restart_without_private_changes() {
    let directory = TempDir::new().unwrap();
    let host = AgentHost::open(directory.path()).unwrap();
    let owner = AgentId::Janet;
    let mut fact = json!({"content":"Orchard uses Rust", "category":"project", "confidence":0.9,
        "source_type":"user", "source_ref":"message:reviewed", "sensitivity":"ordinary", "expires_at":null});
    let saved = operate(&host, owner, json!({"operation":"memory.save","fact":fact})).await;
    let records = operate(&host, owner, json!({"operation":"memory.page"})).await;
    let record = &records["items"][0];
    let shared = operate(
        &host,
        owner,
        json!({"operation":"memory.share","id":saved["id"],"reviewed_hash":record["review_hash"]}),
    )
    .await;
    let records = operate(&host, owner, json!({"operation":"shared.page"})).await;
    let record = &records["items"][0];
    fact["content"] = json!("Orchard uses reviewed Rust tools");
    let edit = json!({"operation":"shared.save","id":shared["id"],"reviewed_hash":record["review_hash"],"fact":fact});
    assert!(host
        .operate(&Config::default(), AgentId::Harley, edit.clone())
        .await
        .is_err());
    operate(&host, owner, edit.clone()).await;
    assert!(host.operate(&Config::default(), owner, edit).await.is_err());
    drop(host);
    let host = AgentHost::open(directory.path()).unwrap();
    let records = operate(&host, owner, json!({"operation":"shared.page"})).await;
    let record = &records["items"][0];
    assert_eq!(record["content"], fact["content"]);
    operate(&host, owner, json!({"operation":"shared.verify","id":shared["id"],"reviewed_hash":record["review_hash"]})).await;
    let private = operate(&host, owner, json!({"operation":"memory.page"})).await;
    assert_eq!(private["items"][0]["content"], "Orchard uses Rust");
    let peer = operate(&host, AgentId::Harley, json!({"operation":"memory.page"})).await;
    assert_eq!(peer["items"], json!([]));
    operate(
        &host,
        owner,
        json!({"operation":"memory.unshare","id":shared["id"]}),
    )
    .await;
    let records = operate(&host, owner, json!({"operation":"shared.page"})).await;
    assert_eq!(records["items"], json!([]));
}

#[tokio::test]
async fn peer_contract_requires_separate_exact_human_reviews_and_closes_after_delivery() {
    let directory = TempDir::new().unwrap();
    let host = AgentHost::open(directory.path()).unwrap();
    let task = host
        .create_task(
            AgentId::Janet,
            "Coordinate",
            "Review one shared detail",
            None,
        )
        .unwrap();
    let request = operate(&host, AgentId::Janet, json!({"operation":"cross.request","recipient":"harley","task_id":task.id,"question":"Which hour?"})).await;
    assert_eq!(
        operate(&host, AgentId::Harley, json!({"operation":"cross.list"})).await,
        json!([])
    );
    let answer = json!({"operation":"cross.respond","id":request["id"],"response":"10 AM"});
    assert!(host
        .operate(&Config::default(), AgentId::Harley, answer.clone())
        .await
        .is_err());
    let queue = operate(
        &host,
        AgentId::Janet,
        json!({"operation":"cross.review_queue"}),
    )
    .await;
    operate(&host, AgentId::Janet, json!({"operation":"cross.review","id":request["id"],"reviewed_hash":queue[0]["question_hash"],"decision":"approve_question"})).await;
    operate(&host, AgentId::Harley, answer.clone()).await;
    assert!(host
        .operate(&Config::default(), AgentId::Harley, answer.clone())
        .await
        .is_err());
    drop(host);
    let host = AgentHost::open(directory.path()).unwrap();
    let requester = operate(&host, AgentId::Janet, json!({"operation":"cross.list"})).await;
    assert!(requester[0]["response"].is_null());
    let queue = operate(
        &host,
        AgentId::Janet,
        json!({"operation":"cross.review_queue"}),
    )
    .await;
    operate(&host, AgentId::Janet, json!({"operation":"cross.review","id":request["id"],"reviewed_hash":queue[0]["response_hash"],"decision":"approve_response"})).await;
    let requester = operate(&host, AgentId::Janet, json!({"operation":"cross.list"})).await;
    assert_eq!(requester[0]["response"], "10 AM");
    assert_eq!(requester[0]["status"], "closed");
    assert!(host
        .operate(&Config::default(), AgentId::Harley, answer)
        .await
        .is_err());
    assert!(host.memories(AgentId::Janet, "").unwrap().is_empty());
    assert!(host.memories(AgentId::Harley, "").unwrap().is_empty());
}
