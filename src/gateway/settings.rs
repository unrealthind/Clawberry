//! Dedicated human configuration API; payloads never enter operation/model logs.
use super::{assistant_authenticated, AppState};
use crate::assistants::{
    settings::{AgentSettings, GlobalSettings, Versioned},
    AgentId,
};
use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/settings/global", get(global_get).post(global_save))
        .route(
            "/api/agents/:agent/settings",
            get(agent_get).post(agent_save),
        )
        .route("/api/runtime/status", get(status))
        .route(
            "/api/settings/claude-credential",
            axum::routing::post(credential_save).layer(axum::extract::DefaultBodyLimit::max(8192)),
        )
}
fn reply<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    let mut response=match result {
        Ok(value)=>(StatusCode::OK,Json(value)).into_response(),
        Err(error) if error.to_string().starts_with("Settings changed elsewhere") => (StatusCode::CONFLICT,Json(serde_json::json!({"error":"Settings changed elsewhere. Reload before saving; your draft is preserved."}))).into_response(),
        Err(_)=>(StatusCode::BAD_REQUEST,Json(serde_json::json!({"error":"Settings could not be read or saved. Check the values and runtime storage; your draft is preserved."}))).into_response(),
    };
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
    response
}
async fn global_get(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if !assistant_authenticated(&state, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let (Some(host), Some(config)) = (&state.assistants, &state.assistant_config) else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    reply(host.global_settings(config))
}
async fn global_save(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Result<Json<Versioned<GlobalSettings>>, axum::extract::rejection::JsonRejection>,
) -> Response {
    if !assistant_authenticated(&state, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let Some(host) = &state.assistants else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let Ok(Json(input)) = body else {
        return reply::<serde_json::Value>(Err(anyhow::anyhow!("Invalid settings")));
    };
    reply(host.save_global_settings(input))
}
async fn agent_get(
    State(state): State<AppState>,
    Path(agent): Path<String>,
    headers: HeaderMap,
) -> Response {
    if !assistant_authenticated(&state, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let (Some(host), Ok(owner)) = (&state.assistants, AgentId::parse(&agent)) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    reply(host.agent_settings(owner))
}
async fn agent_save(
    State(state): State<AppState>,
    Path(agent): Path<String>,
    headers: HeaderMap,
    body: Result<Json<Versioned<AgentSettings>>, axum::extract::rejection::JsonRejection>,
) -> Response {
    if !assistant_authenticated(&state, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let (Some(host), Ok(owner)) = (&state.assistants, AgentId::parse(&agent)) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Ok(Json(input)) = body else {
        return reply::<serde_json::Value>(Err(anyhow::anyhow!("Invalid settings")));
    };
    reply(host.save_agent_settings(owner, input))
}
async fn status(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if !assistant_authenticated(&state, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let (Some(host), Some(config)) = (&state.assistants, &state.assistant_config) else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    reply(host.runtime_status(config))
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CredentialInput {
    token: String,
}
async fn credential_save(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Result<Json<CredentialInput>, axum::extract::rejection::JsonRejection>,
) -> Response {
    if !assistant_authenticated(&state, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let Some(config) = &state.assistant_config else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let Ok(Json(input)) = body else {
        return reply::<serde_json::Value>(Err(anyhow::anyhow!("Invalid credential")));
    };
    reply(
        crate::security::secrets::save_claude_credential(config, &input.token).and_then(|()| {
            if let Some(host) = &state.assistants {
                host.audit(
                    &uuid::Uuid::new_v4().to_string(),
                    AgentId::Janet,
                    "provider_credential_updated",
                    "Shared Claude credential replaced through human settings",
                )?;
            }
            Ok(serde_json::json!({"configured":true,"source":"secure_store"}))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assistants::AgentHost;
    use crate::config::Config;
    #[tokio::test]
    async fn dedicated_settings_api_requires_human_auth_and_rejects_secret_fields() {
        let root = tempfile::TempDir::new().unwrap();
        let mut state = super::super::tests::assistant_test_state(true);
        let host = AgentHost::open(root.path()).unwrap();
        let config = Config {
            config_path: root.path().join("config.toml"),
            default_provider: Some("anthropic".into()),
            default_model: Some("claude-sonnet-5-5".into()),
            ..Config::default()
        };
        state.assistants = Some(std::sync::Arc::new(host));
        state.assistant_config = Some(std::sync::Arc::new(config));
        assert_eq!(
            global_get(State(state.clone()), HeaderMap::new())
                .await
                .status(),
            StatusCode::UNAUTHORIZED
        );
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            "Bearer test-owner-token".parse().unwrap(),
        );
        let mut global = state
            .assistants
            .as_ref()
            .unwrap()
            .global_settings(state.assistant_config.as_ref().unwrap())
            .unwrap();
        global.value.preferences.theme = "light".into();
        assert_eq!(
            global_save(
                State(state.clone()),
                headers.clone(),
                Ok(Json(global.clone()))
            )
            .await
            .status(),
            StatusCode::OK
        );
        assert_eq!(
            global_save(State(state.clone()), headers.clone(), Ok(Json(global)))
                .await
                .status(),
            StatusCode::CONFLICT
        );
        let mut agent = state
            .assistants
            .as_ref()
            .unwrap()
            .agent_settings(AgentId::Janet)
            .unwrap();
        agent.value.memory_notes = "sk-ant-fixture-secret".into();
        assert_eq!(
            agent_save(State(state), Path("janet".into()), headers, Ok(Json(agent)))
                .await
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
}

#[cfg(test)]
mod credential_tests {
    use super::*;
    #[tokio::test]
    async fn enrollment_requires_authentication_and_returns_no_secret() {
        let root = tempfile::tempdir().unwrap();
        let mut state = super::super::tests::assistant_test_state(true);
        state.assistant_config = Some(std::sync::Arc::new(crate::config::Config {
            config_path: root.path().join("config.toml"),
            ..Default::default()
        }));
        let input = || {
            Ok(Json(CredentialInput {
                token: "sk-ant-api03-fixture-enrollment-token".into(),
            }))
        };
        assert_eq!(
            credential_save(State(state.clone()), HeaderMap::new(), input())
                .await
                .status(),
            StatusCode::UNAUTHORIZED
        );
        assert!(!root.path().join("claude-credential.enc").exists());
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            "Bearer test-owner-token".parse().unwrap(),
        );
        let response = credential_save(State(state), headers, input()).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        assert!(!String::from_utf8_lossy(&body).contains("sk-ant-"));
    }
}
