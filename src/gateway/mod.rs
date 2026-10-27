//! Axum-based HTTP gateway with proper HTTP/1.1 compliance, body limits, and timeouts.
//!
//! This module replaces the raw TCP implementation with axum for:
//! - Proper HTTP/1.1 parsing and compliance
//! - Content-Length validation (handled by hyper)
//! - Request body size limits (64KB max)
//! - Request timeouts (30s) to prevent slow-loris attacks
//! - Header sanitization (handled by axum/hyper)

mod assets;
mod settings;

use crate::assistants::{AgentHost, AgentId};
use crate::channels::{Channel, WhatsAppChannel};
use crate::config::Config;
use crate::memory::{self, Memory, MemoryCategory};
use crate::providers::{self, Provider};
use crate::security::pairing::{constant_time_eq, is_public_bind, PairingGuard};
use crate::util::truncate_with_ellipsis;
use anyhow::Result;
use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;

/// Maximum request body size (64KB) — prevents memory exhaustion
pub const MAX_BODY_SIZE: usize = 65_536;
/// Request timeout (30s) — prevents slow-loris attacks
pub const REQUEST_TIMEOUT_SECS: u64 = 30;
/// Sliding window used by gateway rate limiting.
pub const RATE_LIMIT_WINDOW_SECS: u64 = 60;

#[derive(Debug)]
struct SlidingWindowRateLimiter {
    limit_per_window: u32,
    window: Duration,
    requests: Mutex<HashMap<String, Vec<Instant>>>,
}

impl SlidingWindowRateLimiter {
    fn new(limit_per_window: u32, window: Duration) -> Self {
        Self {
            limit_per_window,
            window,
            requests: Mutex::new(HashMap::new()),
        }
    }

    fn allow(&self, key: &str) -> bool {
        if self.limit_per_window == 0 {
            return true;
        }

        let now = Instant::now();
        let cutoff = now.checked_sub(self.window).unwrap_or_else(Instant::now);

        let mut requests = self
            .requests
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        let entry = requests.entry(key.to_owned()).or_default();
        entry.retain(|instant| *instant > cutoff);

        if entry.len() >= self.limit_per_window as usize {
            return false;
        }

        entry.push(now);
        true
    }
}

#[derive(Debug)]
pub struct GatewayRateLimiter {
    pair: SlidingWindowRateLimiter,
    webhook: SlidingWindowRateLimiter,
    write_authorization: SlidingWindowRateLimiter,
}

impl GatewayRateLimiter {
    fn new(pair_per_minute: u32, webhook_per_minute: u32) -> Self {
        let window = Duration::from_secs(RATE_LIMIT_WINDOW_SECS);
        Self {
            pair: SlidingWindowRateLimiter::new(pair_per_minute, window),
            webhook: SlidingWindowRateLimiter::new(webhook_per_minute, window),
            write_authorization: SlidingWindowRateLimiter::new(5, window),
        }
    }

    fn allow_pair(&self, key: &str) -> bool {
        self.pair.allow(key)
    }

    fn allow_webhook(&self, key: &str) -> bool {
        self.webhook.allow(key)
    }
}

#[derive(Debug)]
pub struct IdempotencyStore {
    ttl: Duration,
    keys: Mutex<HashMap<String, Instant>>,
}

impl IdempotencyStore {
    fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            keys: Mutex::new(HashMap::new()),
        }
    }

    /// Returns true if this key is new and is now recorded.
    fn record_if_new(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut keys = self
            .keys
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        keys.retain(|_, seen_at| now.duration_since(*seen_at) < self.ttl);

        if keys.contains_key(key) {
            return false;
        }

        keys.insert(key.to_owned(), now);
        true
    }
}

fn client_key_from_headers(headers: &HeaderMap, trust_proxy_headers: bool) -> String {
    if trust_proxy_headers {
        for header_name in ["X-Forwarded-For", "X-Real-IP"] {
            if let Some(value) = headers.get(header_name).and_then(|v| v.to_str().ok()) {
                let first = value.split(',').next().unwrap_or("").trim();
                if !first.is_empty() {
                    return first.to_owned();
                }
            }
        }
    }
    // Axum handlers in this gateway do not currently carry ConnectInfo. Using
    // a single conservative bucket is safer than trusting attacker-controlled
    // forwarding headers. Deployments that need per-client proxy limiting must
    // explicitly opt into trusted proxy headers and prevent direct gateway access.
    "gateway".into()
}

/// Shared state for all axum handlers
#[derive(Clone)]
pub struct AppState {
    pub assistants: Option<Arc<AgentHost>>,
    pub assistant_config: Option<Arc<Config>>,
    pub provider: Arc<dyn Provider>,
    pub model: String,
    pub temperature: f64,
    pub mem: Arc<dyn Memory>,
    pub auto_save: bool,
    pub webhook_secret: Option<Arc<str>>,
    pub pairing: Arc<PairingGuard>,
    pub local_ui: bool,
    pub rate_limiter: Arc<GatewayRateLimiter>,
    pub idempotency_store: Arc<IdempotencyStore>,
    pub trust_proxy_headers: bool,
    pub whatsapp: Option<Arc<WhatsAppChannel>>,
    /// `WhatsApp` app secret for webhook signature verification (`X-Hub-Signature-256`)
    pub whatsapp_app_secret: Option<Arc<str>>,
}

/// Run the HTTP gateway using axum with proper HTTP/1.1 compliance.
#[allow(clippy::too_many_lines)]
pub async fn run_gateway(host: &str, port: u16, config: Config) -> Result<()> {
    // ── Security: refuse public bind without tunnel or explicit opt-in ──
    if is_public_bind(host) && config.tunnel.provider == "none" && !config.gateway.allow_public_bind
    {
        anyhow::bail!(
            "🛑 Refusing to bind to {host} — gateway would be exposed to the internet.\n\
             Fix: use --host 127.0.0.1 (default), configure a tunnel, or set\n\
             [gateway] allow_public_bind = true in config.toml (NOT recommended)."
        );
    }

    let addr: SocketAddr = format!("{host}:{port}").parse()?;
    validate_local_ui(&config, addr)?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let actual_port = listener.local_addr()?.port();
    let display_addr = format!("{host}:{actual_port}");

    let provider: Arc<dyn Provider> = Arc::from(providers::create_resilient_provider(
        config.default_provider.as_deref().unwrap_or("openrouter"),
        config.api_key.as_deref(),
        &config.reliability,
    )?);
    let model = config
        .default_model
        .clone()
        .unwrap_or_else(|| "anthropic/claude-sonnet-4-20250514".into());
    let temperature = config.default_temperature;
    let mem: Arc<dyn Memory> = Arc::from(memory::create_memory(
        &config.memory,
        &config.workspace_dir,
        config.api_key.as_deref(),
    )?);

    // Extract webhook secret for authentication
    let webhook_secret: Option<Arc<str>> = config
        .channels_config
        .webhook
        .as_ref()
        .and_then(|w| w.secret.as_deref())
        .map(Arc::from);

    // WhatsApp channel (if configured)
    let whatsapp_channel: Option<Arc<WhatsAppChannel>> =
        config.channels_config.whatsapp.as_ref().map(|wa| {
            Arc::new(WhatsAppChannel::new(
                wa.access_token.clone(),
                wa.phone_number_id.clone(),
                wa.verify_token.clone(),
                wa.allowed_numbers.clone(),
            ))
        });

    // WhatsApp app secret for webhook signature verification
    // Priority: environment variable > config file
    let whatsapp_app_secret: Option<Arc<str>> = std::env::var("ZEROCLAW_WHATSAPP_APP_SECRET")
        .ok()
        .and_then(|secret| {
            let secret = secret.trim();
            (!secret.is_empty()).then(|| secret.to_owned())
        })
        .or_else(|| {
            config.channels_config.whatsapp.as_ref().and_then(|wa| {
                wa.app_secret
                    .as_deref()
                    .map(str::trim)
                    .filter(|secret| !secret.is_empty())
                    .map(ToOwned::to_owned)
            })
        })
        .map(Arc::from);

    // ── Pairing guard ──────────────────────────────────────
    let pairing = Arc::new(PairingGuard::new(
        config.gateway.require_pairing,
        &config.gateway.paired_tokens,
    ));
    let rate_limiter = Arc::new(GatewayRateLimiter::new(
        config.gateway.pair_rate_limit_per_minute,
        config.gateway.webhook_rate_limit_per_minute,
    ));
    let idempotency_store = Arc::new(IdempotencyStore::new(Duration::from_secs(
        config.gateway.idempotency_ttl_secs.max(1),
    )));

    // ── Tunnel ────────────────────────────────────────────────
    let tunnel = crate::tunnel::create_tunnel(&config.tunnel)?;
    let mut tunnel_url: Option<String> = None;

    if let Some(ref tun) = tunnel {
        println!("🔗 Starting {} tunnel...", tun.name());
        match tun.start(host, actual_port).await {
            Ok(url) => {
                println!("🌐 Tunnel active: {url}");
                tunnel_url = Some(url);
            }
            Err(e) => {
                println!("⚠️  Tunnel failed to start: {e}");
                println!("   Falling back to local-only mode.");
            }
        }
    }

    println!("RaaviClaw Gateway listening on http://{display_addr}");
    if let Some(ref url) = tunnel_url {
        println!("  🌐 Public URL: {url}");
    }
    println!("  POST /pair      — pair a new client (X-Pairing-Code header)");
    println!("  POST /webhook   — {{\"message\": \"your prompt\"}}");
    if whatsapp_channel.is_some() {
        println!("  GET  /whatsapp  — Meta webhook verification");
        println!("  POST /whatsapp  — WhatsApp message webhook");
    }
    println!("  GET  /health    — health check");
    if config.gateway.local_ui {
        println!("  Local UI: direct loopback access, no gateway token required");
    } else if let Some(code) = pairing.pairing_code() {
        println!();
        println!("  🔐 PAIRING REQUIRED — use this one-time code:");
        println!("     ┌──────────────┐");
        println!("     │  {code}  │");
        println!("     └──────────────┘");
        println!("     Send: POST /pair with header X-Pairing-Code: {code}");
    } else if pairing.require_pairing() {
        println!("  🔒 Pairing: ACTIVE (bearer token required)");
    } else {
        println!("  ⚠️  Pairing: DISABLED (all requests accepted)");
    }
    if webhook_secret.is_some() {
        println!("  🔒 Webhook secret: ENABLED");
    }
    println!("  Press Ctrl+C to stop.\n");

    crate::health::mark_component_ok("gateway");

    // Build the permanent-agent host before publishing HTTP state. If guarded
    // execution is configured, reconcile stale sandbox containers left by an
    // unclean prior process before accepting any requests.
    let assistants = Arc::new(AgentHost::open(&config.workspace_dir.join("assistants"))?);
    let execution_path = std::path::Path::new("/etc/tinyclaw/security/execution.toml");
    if execution_path.exists() {
        let execution = AgentHost::execution_config()?;
        let removed = execution.sandbox.cleanup_orphans().await?;
        if removed > 0 {
            tracing::warn!(
                removed,
                "Removed stale RaaviClaw sandbox containers at startup"
            );
        }
    }

    // Build shared state
    let state = AppState {
        assistants: Some(assistants),
        assistant_config: Some(Arc::new(config.clone())),
        provider,
        model,
        temperature,
        mem,
        auto_save: config.memory.auto_save,
        webhook_secret,
        pairing,
        local_ui: config.gateway.local_ui,
        rate_limiter,
        idempotency_store,
        trust_proxy_headers: config.gateway.trust_proxy_headers,
        whatsapp: whatsapp_channel,
        whatsapp_app_secret,
    };

    // The same host owns model worker lifetimes and watch leases. Schedules are
    // disabled by default; only the human API can enable them independently.
    let _watch_loop = state
        .assistants
        .as_ref()
        .zip(state.assistant_config.as_ref())
        .map(|(host, config)| {
            let (host, config) = (Arc::clone(host), Arc::clone(config));
            WatchLoop(tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(30));
                loop {
                    interval.tick().await;
                    for owner in [AgentId::Janet, AgentId::Harley] {
                        if let Err(error) = host.watch_tick(&config, owner) {
                            tracing::error!(agent = owner.as_str(), "Watch tick failed: {error}");
                        }
                    }
                }
            }))
        });

    // Build router with middleware
    let app = Router::new()
        .route("/health", get(handle_health))
        .route("/pair", post(handle_pair))
        .route("/api/agents", get(handle_agents))
        .route("/api/chat/:agent", post(handle_assistant_chat))
        .route("/api/chat/:agent/stream", post(handle_assistant_stream))
        .route(
            "/api/agents/:agent/write-authorizations",
            post(handle_write_authorization),
        )
        .route(
            "/api/agents/:agent/operations",
            post(handle_assistant_operation),
        )
        .route("/webhook", post(handle_webhook))
        .route("/whatsapp", get(handle_whatsapp_verify))
        .route("/whatsapp", post(handle_whatsapp_message))
        .layer(RequestBodyLimitLayer::new(MAX_BODY_SIZE))
        .merge(
            Router::new()
                .route(
                    "/api/agents/:agent/threads/:thread/uploads",
                    post(handle_upload),
                )
                .route("/api/agents/:agent/files/:id", get(handle_file))
                .layer(axum::extract::DefaultBodyLimit::max(5_000_000))
                .layer(RequestBodyLimitLayer::new(5_000_000)),
        )
        .merge(
            settings::routes()
                .layer(axum::extract::DefaultBodyLimit::max(700_000))
                .layer(RequestBodyLimitLayer::new(700_000)),
        )
        .fallback(get(assets::serve))
        .with_state(state)
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(REQUEST_TIMEOUT_SECS),
        ));

    // Private responses (including errors/pairing) must not enter browser or
    // intermediary caches. Static assets already use the same development policy.
    let app = app.layer(axum::middleware::map_response(no_store_response));
    // Run the server
    axum::serve(listener, app).await?;

    Ok(())
}

async fn no_store_response(mut response: axum::response::Response) -> axum::response::Response {
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
    response
}

struct WatchLoop(tokio::task::JoinHandle<()>);
impl Drop for WatchLoop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

// Token-free access is an explicit host startup mode, never an agent/UI setting.
fn validate_local_ui(config: &Config, addr: SocketAddr) -> Result<()> {
    if config.gateway.local_ui
        && (!addr.ip().is_loopback()
            || config.tunnel.provider != "none"
            || config.gateway.trust_proxy_headers)
    {
        anyhow::bail!("Local UI requires a loopback bind, no tunnel, and no trusted proxy headers");
    }
    Ok(())
}

fn loopback_hostname(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host
            .trim_matches(['[', ']'])
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
}

fn local_ui_request(headers: &HeaderMap) -> bool {
    let local_host = headers
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<axum::http::uri::Authority>().ok())
        .is_some_and(|authority| loopback_hostname(authority.host()));
    if !local_host
        || [
            "forwarded",
            "x-forwarded-for",
            "x-forwarded-host",
            "x-real-ip",
        ]
        .iter()
        .any(|name| headers.contains_key(*name))
    {
        return false;
    }
    if let Some(site) = headers.get("sec-fetch-site") {
        if !matches!(site.to_str(), Ok("same-origin" | "same-site" | "none")) {
            return false;
        }
    }
    match headers.get(header::ORIGIN) {
        None => true, // Direct CLI clients do not send browser Origin metadata.
        Some(origin) => origin
            .to_str()
            .ok()
            .and_then(|v| reqwest::Url::parse(v).ok())
            .is_some_and(|origin| {
                matches!(origin.scheme(), "http" | "https")
                    && origin.host_str().is_some_and(loopback_hostname)
                    && origin.username().is_empty()
                    && origin.password().is_none()
                    && origin.path() == "/"
                    && origin.query().is_none()
                    && origin.fragment().is_none()
            }),
    }
}

// Disabling legacy webhook pairing alone never opens the assistant API.
fn assistant_authenticated(state: &AppState, headers: &HeaderMap) -> bool {
    (state.local_ui && local_ui_request(headers))
        || (state.pairing.require_pairing()
            && headers
                .get(header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok())
                .and_then(|value| value.strip_prefix("Bearer "))
                .is_some_and(|token| state.pairing.is_authenticated(token)))
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct UploadQuery {
    filename: String,
}

async fn handle_upload(
    State(state): State<AppState>,
    Path((agent, thread)): Path<(String, String)>,
    headers: HeaderMap,
    Query(query): Query<UploadQuery>,
    body: Bytes,
) -> axum::response::Response {
    if !assistant_authenticated(&state, &headers) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error":"Authenticated pairing required"})),
        )
            .into_response();
    }
    if !state.rate_limiter.allow_webhook(&client_key_from_headers(
        &headers,
        state.trust_proxy_headers,
    )) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(serde_json::json!({"error":"Rate limit exceeded"})),
        )
            .into_response();
    }
    let (Ok(owner), Some(host)) = (AgentId::parse(&agent), &state.assistants) else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error":"Assistant unavailable"})),
        )
            .into_response();
    };
    match host.put_attachment(owner, &thread, None, &query.filename, &body) {
        Ok(file) => (StatusCode::CREATED, Json(file)).into_response(),
        Err(_) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error":"Invalid upload or ownership scope"})),
        )
            .into_response(),
    }
}

async fn handle_file(
    State(state): State<AppState>,
    Path((agent, id)): Path<(String, String)>,
    headers: HeaderMap,
) -> axum::response::Response {
    if !assistant_authenticated(&state, &headers) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error":"Authenticated pairing required"})),
        )
            .into_response();
    }
    if !state.rate_limiter.allow_webhook(&client_key_from_headers(
        &headers,
        state.trust_proxy_headers,
    )) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(serde_json::json!({"error":"Rate limit exceeded"})),
        )
            .into_response();
    }
    let (Ok(owner), Some(host)) = (AgentId::parse(&agent), &state.assistants) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    match host.attachment_bytes(owner, &id) {
        Ok(bytes) => (
            [
                (header::CONTENT_TYPE, "application/octet-stream"),
                (header::CONTENT_DISPOSITION, "attachment"),
                (header::CACHE_CONTROL, "no-store"),
                (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            ],
            bytes,
        )
            .into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn handle_agents(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    if !assistant_authenticated(&state, &headers) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Authenticated pairing required"})),
        );
    }
    match &state.assistants {
        Some(host) => (
            StatusCode::OK,
            Json(serde_json::json!({"agents": host.records()})),
        ),
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "Assistant host unavailable"})),
        ),
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct AssistantChatBody {
    message: String,
    #[serde(default)]
    thread_id: Option<String>,
    #[serde(default)]
    task_id: Option<String>,
    #[serde(default)]
    profile: Option<crate::assistants::models::Profile>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct WriteAuthorizationBody {
    review_id: String,
    reviewed_hash: String,
    code: String,
}

// Dedicated human-only path: never forwards body to chat/operation persistence.
async fn handle_write_authorization(
    State(state): State<AppState>,
    Path(agent): Path<String>,
    headers: HeaderMap,
    body: std::result::Result<
        Json<WriteAuthorizationBody>,
        axum::extract::rejection::JsonRejection,
    >,
) -> impl IntoResponse {
    let response = if !assistant_authenticated(&state, &headers) {
        (
            StatusCode::UNAUTHORIZED,
            serde_json::json!({"error":"Authenticated pairing required"}),
        )
    } else if !state
        .rate_limiter
        .write_authorization
        .allow("authenticated-human")
    {
        (
            StatusCode::TOO_MANY_REQUESTS,
            serde_json::json!({"error":"Rate limit exceeded"}),
        )
    } else if let (Ok(owner), Ok(Json(body)), Some(host)) =
        (AgentId::parse(&agent), body, state.assistants)
    {
        let result = tokio::task::spawn_blocking(move || {
            host.authorize_write(owner, &body.review_id, &body.reviewed_hash, &body.code)
        })
        .await;
        match result {
            Ok(Ok(value)) => (StatusCode::OK, value),
            _ => (
                StatusCode::BAD_REQUEST,
                serde_json::json!({"error":"Authorization failed; prepare a fresh review before retrying"}),
            ),
        }
    } else {
        (
            StatusCode::BAD_REQUEST,
            serde_json::json!({"error":"Invalid authorization request"}),
        )
    };
    (
        response.0,
        [(header::CACHE_CONTROL, "no-store")],
        Json(response.1),
    )
}

async fn handle_assistant_operation(
    State(state): State<AppState>,
    Path(agent): Path<String>,
    headers: HeaderMap,
    body: std::result::Result<Json<serde_json::Value>, axum::extract::rejection::JsonRejection>,
) -> impl IntoResponse {
    if !assistant_authenticated(&state, &headers) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error":"Authenticated pairing required"})),
        );
    }
    if !state.rate_limiter.allow_webhook(&client_key_from_headers(
        &headers,
        state.trust_proxy_headers,
    )) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(serde_json::json!({"error":"Rate limit exceeded"})),
        );
    }
    let Ok(owner) = AgentId::parse(&agent) else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error":"Unknown permanent agent"})),
        );
    };
    let Ok(Json(payload)) = body else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error":"Invalid operation JSON"})),
        );
    };
    let (Some(host), Some(config)) = (&state.assistants, &state.assistant_config) else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error":"Assistant host unavailable"})),
        );
    };
    match host.operate(config, owner, payload).await {
        Ok(result) => (StatusCode::OK, Json(result)),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(
                serde_json::json!({"error":crate::providers::sanitize_api_error(&error.to_string())}),
            ),
        ),
    }
}

async fn handle_assistant_stream(
    State(state): State<AppState>,
    Path(agent): Path<String>,
    headers: HeaderMap,
    body: std::result::Result<Json<AssistantChatBody>, axum::extract::rejection::JsonRejection>,
) -> axum::response::Response {
    if !assistant_authenticated(&state, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    if !state.rate_limiter.allow_webhook(&client_key_from_headers(
        &headers,
        state.trust_proxy_headers,
    )) {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    }
    let Ok(owner) = AgentId::parse(&agent) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Ok(Json(body)) = body else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    if body.message.trim().is_empty() || body.message.len() > 32_000 {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let (Some(host), Some(config)) = (state.assistants, state.assistant_config) else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let Ok(saved_settings) = host.agent_settings(owner) else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let profile = body.profile.unwrap_or(saved_settings.value.default_profile);
    let (events, received) = tokio::sync::mpsc::channel::<serde_json::Value>(32);
    tokio::spawn(async move {
        let (text, mut deltas) = tokio::sync::mpsc::channel(32);
        let turn = host.chat_stream(
            &config,
            owner,
            body.thread_id.as_deref(),
            body.task_id.as_deref(),
            profile,
            &body.message,
            Some(text),
        );
        tokio::pin!(turn);
        let result = loop {
            tokio::select! {
                () = events.closed() => {
                    let _ = host.audit(&uuid::Uuid::new_v4().to_string(),owner,"chat_stream_cancelled","Partial text not committed; model billing may be uncertain");
                    return;
                }
                delta = deltas.recv() => if let Some(text) = delta {
                    if events.send(serde_json::json!({"type":"delta","text":text})).await.is_err() { return; }
                },
                result = &mut turn => break result,
            }
        };
        while let Ok(text) = deltas.try_recv() {
            if events
                .send(serde_json::json!({"type":"delta","text":text}))
                .await
                .is_err()
            {
                return;
            }
        }
        let event = match result {
            Ok(response) => serde_json::json!({"type":"complete","response":response}),
            Err(error) => {
                let code = crate::assistants::failures::code(&error);
                tracing::warn!(
                    agent = owner.as_str(),
                    failure_code = code,
                    "Assistant stream failed"
                );
                serde_json::json!({"type":"error","code":code,"error":"Chat failed. Refresh history and inspect activity before retrying; completed internal operations remain recorded."})
            }
        };
        let _ = events.send(event).await;
    });
    let stream = futures_util::stream::unfold(received, |mut receiver| async move {
        receiver.recv().await.map(|value| {
            (
                Ok::<_, std::convert::Infallible>(
                    axum::response::sse::Event::default()
                        .json_data(value)
                        .expect("serializable stream event"),
                ),
                receiver,
            )
        })
    });
    let mut response = axum::response::Sse::new(stream)
        .keep_alive(axum::response::sse::KeepAlive::default())
        .into_response();
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
    response
        .headers_mut()
        .insert("X-Accel-Buffering", "no".parse().unwrap());
    response
}

async fn handle_assistant_chat(
    State(state): State<AppState>,
    Path(agent): Path<String>,
    headers: HeaderMap,
    body: std::result::Result<Json<AssistantChatBody>, axum::extract::rejection::JsonRejection>,
) -> impl IntoResponse {
    if !assistant_authenticated(&state, &headers) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Authenticated pairing required"})),
        );
    }
    if !state.rate_limiter.allow_webhook(&client_key_from_headers(
        &headers,
        state.trust_proxy_headers,
    )) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(serde_json::json!({"error": "Rate limit exceeded"})),
        );
    }
    let Ok(id) = AgentId::parse(&agent) else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Unknown permanent agent"})),
        );
    };
    let Ok(Json(body)) = body else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Expected a message-only JSON object"})),
        );
    };
    if body.message.trim().is_empty() || body.message.len() > 32_000 {
        return (
            StatusCode::BAD_REQUEST,
            Json(
                serde_json::json!({"error": "Message must contain 1–32000 bytes of nonempty text"}),
            ),
        );
    }
    let (Some(host), Some(config)) = (&state.assistants, &state.assistant_config) else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "Assistant host unavailable"})),
        );
    };
    let Ok(saved_settings) = host.agent_settings(id) else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error":"Agent settings unavailable"})),
        );
    };
    let profile = body.profile.unwrap_or(saved_settings.value.default_profile);
    if let Ok(response) = host
        .chat_scoped(
            config,
            id,
            body.thread_id.as_deref(),
            body.task_id.as_deref(),
            profile,
            &body.message,
        )
        .await
    {
        (
            StatusCode::OK,
            Json(
                serde_json::json!({"agent": id, "session_id": body.thread_id.as_deref().unwrap_or(&host.record(id).main_session_id), "response": response}),
            ),
        )
    } else {
        tracing::warn!(
            agent = id.as_str(),
            "Assistant chat failed; inspect correlated host audit events"
        );
        (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({"error": "Assistant turn failed; conversation unchanged"})),
        )
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// AXUM HANDLERS
// ══════════════════════════════════════════════════════════════════════════════

/// GET /health — always public (no secrets leaked)
async fn handle_health(State(state): State<AppState>) -> impl IntoResponse {
    let body = serde_json::json!({
        "status": "ok",
        "paired": state.pairing.is_paired(),
        "runtime": crate::health::snapshot_json(),
    });
    Json(body)
}

/// POST /pair — exchange one-time code for bearer token
async fn handle_pair(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    let client_key = client_key_from_headers(&headers, state.trust_proxy_headers);
    if !state.rate_limiter.allow_pair(&client_key) {
        tracing::warn!("/pair rate limit exceeded for key: {client_key}");
        let err = serde_json::json!({
            "error": "Too many pairing requests. Please retry later.",
            "retry_after": RATE_LIMIT_WINDOW_SECS,
        });
        return (StatusCode::TOO_MANY_REQUESTS, Json(err));
    }

    let code = headers
        .get("X-Pairing-Code")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    match state.pairing.try_pair(code) {
        Ok(Some(token)) => {
            if let Some(config) = state.assistant_config.as_ref() {
                let mut persisted = (**config).clone();
                persisted.gateway.paired_tokens = state.pairing.tokens();
                if let Err(error) = persisted.save() {
                    state.pairing.rollback_pair(&token, code);
                    tracing::error!(%error, "Failed to persist pairing token hash");
                    let err = serde_json::json!({
                        "error": "Pairing could not be persisted; retry with the same one-time code."
                    });
                    return (StatusCode::INTERNAL_SERVER_ERROR, Json(err));
                }
            }
            tracing::info!("🔐 New client paired successfully");
            let body = serde_json::json!({
                "paired": true,
                "token": token,
                "message": "Save this token — use it as Authorization: Bearer <token>"
            });
            (StatusCode::OK, Json(body))
        }
        Ok(None) => {
            tracing::warn!("🔐 Pairing attempt with invalid code");
            let err = serde_json::json!({"error": "Invalid pairing code"});
            (StatusCode::FORBIDDEN, Json(err))
        }
        Err(lockout_secs) => {
            tracing::warn!(
                "🔐 Pairing locked out — too many failed attempts ({lockout_secs}s remaining)"
            );
            let err = serde_json::json!({
                "error": format!("Too many failed attempts. Try again in {lockout_secs}s."),
                "retry_after": lockout_secs
            });
            (StatusCode::TOO_MANY_REQUESTS, Json(err))
        }
    }
}

/// Webhook request body
#[derive(serde::Deserialize)]
pub struct WebhookBody {
    pub message: String,
}

/// POST /webhook — main webhook endpoint
async fn handle_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Result<Json<WebhookBody>, axum::extract::rejection::JsonRejection>,
) -> impl IntoResponse {
    let client_key = client_key_from_headers(&headers, state.trust_proxy_headers);
    if !state.rate_limiter.allow_webhook(&client_key) {
        tracing::warn!("/webhook rate limit exceeded for key: {client_key}");
        let err = serde_json::json!({
            "error": "Too many webhook requests. Please retry later.",
            "retry_after": RATE_LIMIT_WINDOW_SECS,
        });
        return (StatusCode::TOO_MANY_REQUESTS, Json(err));
    }

    // ── Bearer token auth (pairing) ──
    if state.pairing.require_pairing() {
        let auth = headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let token = auth.strip_prefix("Bearer ").unwrap_or("");
        if !state.pairing.is_authenticated(token) {
            tracing::warn!("Webhook: rejected — not paired / invalid bearer token");
            let err = serde_json::json!({
                "error": "Unauthorized — pair first via POST /pair, then send Authorization: Bearer <token>"
            });
            return (StatusCode::UNAUTHORIZED, Json(err));
        }
    }

    // ── Webhook secret auth (optional, additional layer) ──
    if let Some(ref secret) = state.webhook_secret {
        let header_val = headers
            .get("X-Webhook-Secret")
            .and_then(|v| v.to_str().ok());
        match header_val {
            Some(val) if constant_time_eq(val, secret.as_ref()) => {}
            _ => {
                tracing::warn!("Webhook: rejected request — invalid or missing X-Webhook-Secret");
                let err = serde_json::json!({"error": "Unauthorized — invalid or missing X-Webhook-Secret header"});
                return (StatusCode::UNAUTHORIZED, Json(err));
            }
        }
    }

    // ── Parse body ──
    let Json(webhook_body) = match body {
        Ok(b) => b,
        Err(e) => {
            let err = serde_json::json!({
                "error": format!("Invalid JSON: {e}. Expected: {{\"message\": \"...\"}}")
            });
            return (StatusCode::BAD_REQUEST, Json(err));
        }
    };

    // ── Idempotency (optional) ──
    if let Some(idempotency_key) = headers
        .get("X-Idempotency-Key")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if !state.idempotency_store.record_if_new(idempotency_key) {
            tracing::info!("Webhook duplicate ignored (idempotency key: {idempotency_key})");
            let body = serde_json::json!({
                "status": "duplicate",
                "idempotent": true,
                "message": "Request already processed for this idempotency key"
            });
            return (StatusCode::OK, Json(body));
        }
    }

    let message = &webhook_body.message;

    if state.auto_save {
        let _ = state
            .mem
            .store("webhook_msg", message, MemoryCategory::Conversation)
            .await;
    }

    match state
        .provider
        .chat(message, &state.model, state.temperature)
        .await
    {
        Ok(response) => {
            let body = serde_json::json!({"response": response, "model": state.model});
            (StatusCode::OK, Json(body))
        }
        Err(e) => {
            tracing::error!(
                "Webhook provider error: {}",
                providers::sanitize_api_error(&e.to_string())
            );
            let err = serde_json::json!({"error": "LLM request failed"});
            (StatusCode::INTERNAL_SERVER_ERROR, Json(err))
        }
    }
}

/// `WhatsApp` verification query params
#[derive(serde::Deserialize)]
pub struct WhatsAppVerifyQuery {
    #[serde(rename = "hub.mode")]
    pub mode: Option<String>,
    #[serde(rename = "hub.verify_token")]
    pub verify_token: Option<String>,
    #[serde(rename = "hub.challenge")]
    pub challenge: Option<String>,
}

/// GET /whatsapp — Meta webhook verification
async fn handle_whatsapp_verify(
    State(state): State<AppState>,
    Query(params): Query<WhatsAppVerifyQuery>,
) -> impl IntoResponse {
    let Some(ref wa) = state.whatsapp else {
        return (StatusCode::NOT_FOUND, "WhatsApp not configured".to_string());
    };

    // Verify the token matches (constant-time comparison to prevent timing attacks)
    let token_matches = params
        .verify_token
        .as_deref()
        .is_some_and(|t| constant_time_eq(t, wa.verify_token()));
    if params.mode.as_deref() == Some("subscribe") && token_matches {
        if let Some(ch) = params.challenge {
            tracing::info!("WhatsApp webhook verified successfully");
            return (StatusCode::OK, ch);
        }
        return (StatusCode::BAD_REQUEST, "Missing hub.challenge".to_string());
    }

    tracing::warn!("WhatsApp webhook verification failed — token mismatch");
    (StatusCode::FORBIDDEN, "Forbidden".to_string())
}

/// Verify `WhatsApp` webhook signature (`X-Hub-Signature-256`).
/// Returns true if the signature is valid, false otherwise.
/// See: <https://developers.facebook.com/docs/graph-api/webhooks/getting-started#verification-requests>
pub fn verify_whatsapp_signature(app_secret: &str, body: &[u8], signature_header: &str) -> bool {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    // Signature format: "sha256=<hex_signature>"
    let Some(hex_sig) = signature_header.strip_prefix("sha256=") else {
        return false;
    };

    // Decode hex signature
    let Ok(expected) = hex::decode(hex_sig) else {
        return false;
    };

    // Compute HMAC-SHA256
    let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(app_secret.as_bytes()) else {
        return false;
    };
    mac.update(body);

    // Constant-time comparison
    mac.verify_slice(&expected).is_ok()
}

/// POST /whatsapp — incoming message webhook
async fn handle_whatsapp_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let Some(ref wa) = state.whatsapp else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "WhatsApp not configured"})),
        );
    };

    // ── Security: Verify X-Hub-Signature-256 if app_secret is configured ──
    if let Some(ref app_secret) = state.whatsapp_app_secret {
        let signature = headers
            .get("X-Hub-Signature-256")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if !verify_whatsapp_signature(app_secret, &body, signature) {
            tracing::warn!(
                "WhatsApp webhook signature verification failed (signature: {})",
                if signature.is_empty() {
                    "missing"
                } else {
                    "invalid"
                }
            );
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Invalid signature"})),
            );
        }
    }

    // Parse JSON body
    let Ok(payload) = serde_json::from_slice::<serde_json::Value>(&body) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid JSON payload"})),
        );
    };

    // Parse messages from the webhook payload
    let messages = wa.parse_webhook_payload(&payload);

    if messages.is_empty() {
        // Acknowledge the webhook even if no messages (could be status updates)
        return (StatusCode::OK, Json(serde_json::json!({"status": "ok"})));
    }

    // Process each message
    for msg in &messages {
        tracing::info!(
            "WhatsApp message from {}: {}",
            msg.sender,
            truncate_with_ellipsis(&msg.content, 50)
        );

        // Auto-save to memory
        if state.auto_save {
            let _ = state
                .mem
                .store(
                    &format!("whatsapp_{}", msg.sender),
                    &msg.content,
                    MemoryCategory::Conversation,
                )
                .await;
        }

        // Call the LLM
        match state
            .provider
            .chat(&msg.content, &state.model, state.temperature)
            .await
        {
            Ok(response) => {
                // Send reply via WhatsApp
                if let Err(e) = wa.send(&response, &msg.sender).await {
                    tracing::error!("Failed to send WhatsApp reply: {e}");
                }
            }
            Err(e) => {
                tracing::error!("LLM error for WhatsApp message: {e:#}");
                let _ = wa
                    .send(
                        "Sorry, I couldn't process your message right now.",
                        &msg.sender,
                    )
                    .await;
            }
        }
    }

    // Acknowledge the webhook
    (StatusCode::OK, Json(serde_json::json!({"status": "ok"})))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{Memory, MemoryCategory, MemoryEntry};
    use crate::providers::Provider;
    use async_trait::async_trait;
    use axum::http::HeaderValue;
    use axum::response::IntoResponse;
    use http_body_util::BodyExt;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn local_ui_is_explicit_and_rejects_remote_browser_requests() {
        let mut state = assistant_test_state(true);
        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, HeaderValue::from_static("127.0.0.1:8080"));
        headers.insert(
            header::ORIGIN,
            HeaderValue::from_static("http://127.0.0.1:5173"),
        );
        assert!(!assistant_authenticated(&state, &headers));
        state.local_ui = true;
        assert!(assistant_authenticated(&state, &headers));
        for origin in [
            "https://evil.example",
            "null",
            "http://localhost.evil.example",
            "file:///",
        ] {
            headers.insert(header::ORIGIN, HeaderValue::from_str(origin).unwrap());
            assert!(!assistant_authenticated(&state, &headers));
        }
        headers.remove(header::ORIGIN);
        headers.insert("sec-fetch-site", HeaderValue::from_static("cross-site"));
        assert!(!assistant_authenticated(&state, &headers));
        headers.remove("sec-fetch-site");
        for host in [
            "evil.example:8080",
            "localhost.evil.example:8080",
            "192.168.1.10:8080",
        ] {
            headers.insert(header::HOST, HeaderValue::from_str(host).unwrap());
            assert!(!assistant_authenticated(&state, &headers));
        }
        for host in ["localhost:8080", "127.0.0.1:8080", "[::1]:8080"] {
            headers.insert(header::HOST, HeaderValue::from_str(host).unwrap());
            assert!(assistant_authenticated(&state, &headers));
        }
        for forwarded in [
            "forwarded",
            "x-forwarded-for",
            "x-forwarded-host",
            "x-real-ip",
        ] {
            headers.insert(forwarded, HeaderValue::from_static("127.0.0.1"));
            assert!(!assistant_authenticated(&state, &headers));
            headers.remove(forwarded);
        }
    }

    #[test]
    fn local_ui_refuses_remote_bind_tunnels_and_proxy_trust() {
        let mut config = Config::default();
        assert!(!config.gateway.local_ui);
        config.gateway.local_ui = true;
        for addr in ["127.0.0.1:8080", "[::1]:8080"] {
            assert!(validate_local_ui(&config, addr.parse().unwrap()).is_ok());
        }
        for addr in ["0.0.0.0:8080", "[::]:8080", "192.168.1.10:8080"] {
            assert!(validate_local_ui(&config, addr.parse().unwrap()).is_err());
        }
        config.gateway.trust_proxy_headers = true;
        assert!(validate_local_ui(&config, "127.0.0.1:8080".parse().unwrap()).is_err());
        config.gateway.trust_proxy_headers = false;
        config.tunnel.provider = "cloudflare".into();
        assert!(validate_local_ui(&config, "127.0.0.1:8080".parse().unwrap()).is_err());
    }

    #[test]
    fn security_body_limit_is_64kb() {
        assert_eq!(MAX_BODY_SIZE, 65_536);
    }

    #[test]
    fn security_timeout_is_30_seconds() {
        assert_eq!(REQUEST_TIMEOUT_SECS, 30);
    }

    #[test]
    fn webhook_body_requires_message_field() {
        let valid = r#"{"message": "hello"}"#;
        let parsed: Result<WebhookBody, _> = serde_json::from_str(valid);
        assert!(parsed.is_ok());
        assert_eq!(parsed.unwrap().message, "hello");

        let missing = r#"{"other": "field"}"#;
        let parsed: Result<WebhookBody, _> = serde_json::from_str(missing);
        assert!(parsed.is_err());
    }

    #[test]
    fn whatsapp_query_fields_are_optional() {
        let q = WhatsAppVerifyQuery {
            mode: None,
            verify_token: None,
            challenge: None,
        };
        assert!(q.mode.is_none());
    }

    #[test]
    fn app_state_is_clone() {
        fn assert_clone<T: Clone>() {}
        assert_clone::<AppState>();
    }

    #[test]
    fn gateway_rate_limiter_blocks_after_limit() {
        let limiter = GatewayRateLimiter::new(2, 2);
        assert!(limiter.allow_pair("127.0.0.1"));
        assert!(limiter.allow_pair("127.0.0.1"));
        assert!(!limiter.allow_pair("127.0.0.1"));
    }

    #[test]
    fn idempotency_store_rejects_duplicate_key() {
        let store = IdempotencyStore::new(Duration::from_secs(30));
        assert!(store.record_if_new("req-1"));
        assert!(!store.record_if_new("req-1"));
        assert!(store.record_if_new("req-2"));
    }

    #[derive(Default)]
    struct MockMemory;

    #[async_trait]
    impl Memory for MockMemory {
        fn name(&self) -> &str {
            "mock"
        }

        async fn store(
            &self,
            _key: &str,
            _content: &str,
            _category: MemoryCategory,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        async fn recall(&self, _query: &str, _limit: usize) -> anyhow::Result<Vec<MemoryEntry>> {
            Ok(Vec::new())
        }

        async fn get(&self, _key: &str) -> anyhow::Result<Option<MemoryEntry>> {
            Ok(None)
        }

        async fn list(
            &self,
            _category: Option<&MemoryCategory>,
        ) -> anyhow::Result<Vec<MemoryEntry>> {
            Ok(Vec::new())
        }

        async fn forget(&self, _key: &str) -> anyhow::Result<bool> {
            Ok(false)
        }

        async fn count(&self) -> anyhow::Result<usize> {
            Ok(0)
        }

        async fn health_check(&self) -> bool {
            true
        }
    }

    #[derive(Default)]
    struct MockProvider {
        calls: AtomicUsize,
    }

    #[async_trait]
    impl Provider for MockProvider {
        async fn chat_with_system(
            &self,
            _system_prompt: Option<&str>,
            _message: &str,
            _model: &str,
            _temperature: f64,
        ) -> anyhow::Result<String> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok("ok".into())
        }
    }

    #[tokio::test]
    async fn assistant_api_requires_auth_even_when_legacy_pairing_is_disabled() {
        for require_pairing in [false, true] {
            let state = assistant_test_state(require_pairing);
            let response = handle_agents(State(state.clone()), HeaderMap::new())
                .await
                .into_response();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
            let response = handle_assistant_chat(
                State(state),
                Path("janet".into()),
                HeaderMap::new(),
                Ok(Json(AssistantChatBody {
                    message: "hello".into(),
                    thread_id: None,
                    task_id: None,
                    profile: None,
                })),
            )
            .await
            .into_response();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        }
    }

    #[tokio::test]
    async fn pairing_persists_only_hashed_bearer_tokens() {
        let d = tempfile::TempDir::new().unwrap();
        let mut state = assistant_test_state(true);
        state.pairing = Arc::new(PairingGuard::new(true, &[]));
        let config = Config {
            workspace_dir: d.path().join("workspace"),
            config_path: d.path().join("config.toml"),
            ..Config::default()
        };
        state.assistant_config = Some(Arc::new(config));

        let code = state.pairing.pairing_code().unwrap();
        let mut headers = HeaderMap::new();
        headers.insert("X-Pairing-Code", HeaderValue::from_str(&code).unwrap());
        let response = handle_pair(State(state.clone()), headers)
            .await
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let saved = std::fs::read_to_string(d.path().join("config.toml")).unwrap();
        let parsed: Config = toml::from_str(&saved).unwrap();
        assert_eq!(parsed.gateway.paired_tokens.len(), 1);
        assert_eq!(parsed.gateway.paired_tokens[0].len(), 64);
        assert!(parsed.gateway.paired_tokens[0]
            .chars()
            .all(|c| c.is_ascii_hexdigit()));
        assert!(!saved.contains("zc_"));
    }

    #[tokio::test]
    async fn totp_endpoint_requires_auth_and_never_persists_the_code() {
        let d = tempfile::TempDir::new().unwrap();
        let host = Arc::new(AgentHost::open(d.path()).unwrap());
        let mut state = assistant_test_state(true);
        state.assistants = Some(host);
        let body = || {
            Ok(Json(WriteAuthorizationBody {
                review_id: "missing-review".into(),
                reviewed_hash: "hash".into(),
                code: "918273".into(),
            }))
        };
        let response = handle_write_authorization(
            State(state.clone()),
            Path("janet".into()),
            HeaderMap::new(),
            body(),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer test-owner-token"),
        );
        let response =
            handle_write_authorization(State(state), Path("janet".into()), headers, body())
                .await
                .into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let db = rusqlite::Connection::open(d.path().join("data/system.db")).unwrap();
        let count: i64 = db
            .query_row("SELECT COUNT(*) FROM operation_events", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
        for entry in std::fs::read_dir(d.path().join("data")).unwrap() {
            let p = entry.unwrap().path();
            if p.is_file() {
                assert!(!std::fs::read(p).unwrap().windows(6).any(|w| w == b"918273"));
            }
        }
    }

    #[tokio::test]
    async fn totp_attempt_limit_cannot_be_reset_with_forwarded_headers() {
        let state = assistant_test_state(true);
        for attempt in 0..6 {
            let mut headers = HeaderMap::new();
            headers.insert(
                header::AUTHORIZATION,
                HeaderValue::from_static("Bearer test-owner-token"),
            );
            headers.insert(
                "x-forwarded-for",
                HeaderValue::from_str(&format!("192.0.2.{attempt}")).unwrap(),
            );
            let response = handle_write_authorization(
                State(state.clone()),
                Path("janet".into()),
                headers,
                Ok(Json(WriteAuthorizationBody {
                    review_id: "missing".into(),
                    reviewed_hash: "hash".into(),
                    code: "918273".into(),
                })),
            )
            .await
            .into_response();
            assert_eq!(
                response.status(),
                if attempt < 5 {
                    StatusCode::BAD_REQUEST
                } else {
                    StatusCode::TOO_MANY_REQUESTS
                }
            );
        }
    }

    pub(super) fn assistant_test_state(require_pairing: bool) -> AppState {
        AppState {
            assistants: None,
            assistant_config: None,
            provider: Arc::new(MockProvider::default()),
            model: "test-model".into(),
            temperature: 0.0,
            mem: Arc::new(MockMemory),
            auto_save: false,
            webhook_secret: None,
            local_ui: false,
            pairing: Arc::new(PairingGuard::new(
                require_pairing,
                &["test-owner-token".into()],
            )),
            rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100)),
            idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300))),
            trust_proxy_headers: false,
            whatsapp: None,
            whatsapp_app_secret: None,
        }
    }

    #[tokio::test]
    async fn authenticated_assistant_api_validates_identity_and_message() {
        let state = assistant_test_state(true);
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer test-owner-token"),
        );
        let response = handle_assistant_chat(
            State(state.clone()),
            Path("researcher".into()),
            headers.clone(),
            Ok(Json(AssistantChatBody {
                message: "hello".into(),
                thread_id: None,
                task_id: None,
                profile: None,
            })),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let response = handle_assistant_chat(
            State(state),
            Path("harley".into()),
            headers,
            Ok(Json(AssistantChatBody {
                message: " ".into(),
                thread_id: None,
                task_id: None,
                profile: None,
            })),
        )
        .await
        .into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(
            serde_json::from_value::<AssistantChatBody>(serde_json::json!({
                "message": "hello", "from": "janet", "session_id": "harley:main"
            }))
            .is_err()
        );
    }

    #[tokio::test]
    async fn authenticated_owner_can_list_both_permanent_agents() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut state = assistant_test_state(true);
        state.assistants = Some(Arc::new(AgentHost::open(dir.path()).unwrap()));
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer test-owner-token"),
        );
        let response = handle_agents(State(state), headers).await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["agents"][0]["id"], "janet");
        assert_eq!(value["agents"][1]["id"], "harley");
    }

    #[tokio::test]
    async fn memory_management_and_activity_require_human_authentication() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut state = assistant_test_state(true);
        state.assistants = Some(Arc::new(AgentHost::open(dir.path()).unwrap()));
        state.assistant_config = Some(Arc::new(Config::default()));
        let payload = serde_json::json!({"operation":"memory.save","id":null,"fact":{
            "content":"Orchard is written in Rust","category":"project","confidence":0.8,"source_type":"user","source_ref":"message:1","sensitivity":"ordinary","expires_at":null
        }});
        let denied = handle_assistant_operation(
            State(state.clone()),
            Path("janet".into()),
            HeaderMap::new(),
            Ok(Json(payload.clone())),
        )
        .await
        .into_response();
        assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer test-owner-token"),
        );
        let saved = handle_assistant_operation(
            State(state.clone()),
            Path("janet".into()),
            headers.clone(),
            Ok(Json(payload)),
        )
        .await
        .into_response();
        assert_eq!(saved.status(), StatusCode::OK);
        assert!(state
            .assistants
            .as_ref()
            .unwrap()
            .memories(AgentId::Harley, "")
            .unwrap()
            .is_empty());
        let response = tokio::time::timeout(
            Duration::from_secs(1),
            handle_assistant_operation(
                State(state),
                Path("janet".into()),
                headers,
                Ok(Json(serde_json::json!({"operation":"activity.list"}))),
            ),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn uploaded_files_require_auth_and_owner_scope() {
        let dir = tempfile::TempDir::new().unwrap();
        let host = Arc::new(AgentHost::open(dir.path()).unwrap());
        let thread = host.create_thread(AgentId::Janet, "Evidence").unwrap();
        let mut state = assistant_test_state(true);
        state.assistants = Some(host.clone());
        let denied = handle_upload(
            State(state.clone()),
            Path(("janet".into(), thread.id.clone())),
            HeaderMap::new(),
            Query(UploadQuery {
                filename: "example.txt".into(),
            }),
            Bytes::from_static(b"test evidence"),
        )
        .await;
        assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer test-owner-token"),
        );
        let saved = handle_upload(
            State(state.clone()),
            Path(("janet".into(), thread.id.clone())),
            headers.clone(),
            Query(UploadQuery {
                filename: "example.txt".into(),
            }),
            Bytes::from_static(b"test evidence"),
        )
        .await;
        assert_eq!(saved.status(), StatusCode::CREATED);
        let file = host
            .attachments(AgentId::Janet, &thread.id)
            .unwrap()
            .remove(0);
        let leaked = handle_file(
            State(state.clone()),
            Path(("harley".into(), file.id.clone())),
            headers.clone(),
        )
        .await;
        assert_eq!(leaked.status(), StatusCode::NOT_FOUND);
        let downloaded = handle_file(State(state), Path(("janet".into(), file.id)), headers).await;
        assert_eq!(downloaded.status(), StatusCode::OK);
        assert_eq!(
            downloaded.headers()[header::X_CONTENT_TYPE_OPTIONS],
            "nosniff"
        );
    }

    #[tokio::test]
    async fn webhook_idempotency_skips_duplicate_provider_calls() {
        let provider_impl = Arc::new(MockProvider::default());
        let provider: Arc<dyn Provider> = provider_impl.clone();
        let memory: Arc<dyn Memory> = Arc::new(MockMemory);

        let state = AppState {
            assistants: None,
            assistant_config: None,
            provider,
            model: "test-model".into(),
            temperature: 0.0,
            mem: memory,
            auto_save: false,
            webhook_secret: None,
            local_ui: false,
            pairing: Arc::new(PairingGuard::new(false, &[])),
            rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100)),
            idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300))),
            trust_proxy_headers: false,
            whatsapp: None,
            whatsapp_app_secret: None,
        };

        let mut headers = HeaderMap::new();
        headers.insert("X-Idempotency-Key", HeaderValue::from_static("abc-123"));

        let body = Ok(Json(WebhookBody {
            message: "hello".into(),
        }));
        let first = handle_webhook(State(state.clone()), headers.clone(), body)
            .await
            .into_response();
        assert_eq!(first.status(), StatusCode::OK);

        let body = Ok(Json(WebhookBody {
            message: "hello".into(),
        }));
        let second = handle_webhook(State(state), headers, body)
            .await
            .into_response();
        assert_eq!(second.status(), StatusCode::OK);

        let payload = second.into_body().collect().await.unwrap().to_bytes();
        let parsed: serde_json::Value = serde_json::from_slice(&payload).unwrap();
        assert_eq!(parsed["status"], "duplicate");
        assert_eq!(parsed["idempotent"], true);
        assert_eq!(provider_impl.calls.load(Ordering::SeqCst), 1);
    }

    // ══════════════════════════════════════════════════════════
    // WhatsApp Signature Verification Tests (CWE-345 Prevention)
    // ══════════════════════════════════════════════════════════

    fn compute_whatsapp_signature_hex(secret: &str, body: &[u8]) -> String {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        hex::encode(mac.finalize().into_bytes())
    }

    fn compute_whatsapp_signature_header(secret: &str, body: &[u8]) -> String {
        format!("sha256={}", compute_whatsapp_signature_hex(secret, body))
    }

    #[test]
    fn whatsapp_signature_valid() {
        // Test with known values
        let app_secret = "test_secret_key";
        let body = b"test body content";

        let signature_header = compute_whatsapp_signature_header(app_secret, body);

        assert!(verify_whatsapp_signature(
            app_secret,
            body,
            &signature_header
        ));
    }

    #[test]
    fn whatsapp_signature_invalid_wrong_secret() {
        let app_secret = "correct_secret";
        let wrong_secret = "wrong_secret";
        let body = b"test body content";

        let signature_header = compute_whatsapp_signature_header(wrong_secret, body);

        assert!(!verify_whatsapp_signature(
            app_secret,
            body,
            &signature_header
        ));
    }

    #[test]
    fn whatsapp_signature_invalid_wrong_body() {
        let app_secret = "test_secret";
        let original_body = b"original body";
        let tampered_body = b"tampered body";

        let signature_header = compute_whatsapp_signature_header(app_secret, original_body);

        // Verify with tampered body should fail
        assert!(!verify_whatsapp_signature(
            app_secret,
            tampered_body,
            &signature_header
        ));
    }

    #[test]
    fn whatsapp_signature_missing_prefix() {
        let app_secret = "test_secret";
        let body = b"test body";

        // Signature without "sha256=" prefix
        let signature_header = "abc123def456";

        assert!(!verify_whatsapp_signature(
            app_secret,
            body,
            signature_header
        ));
    }

    #[test]
    fn whatsapp_signature_empty_header() {
        let app_secret = "test_secret";
        let body = b"test body";

        assert!(!verify_whatsapp_signature(app_secret, body, ""));
    }

    #[test]
    fn whatsapp_signature_invalid_hex() {
        let app_secret = "test_secret";
        let body = b"test body";

        // Invalid hex characters
        let signature_header = "sha256=not_valid_hex_zzz";

        assert!(!verify_whatsapp_signature(
            app_secret,
            body,
            signature_header
        ));
    }

    #[test]
    fn whatsapp_signature_empty_body() {
        let app_secret = "test_secret";
        let body = b"";

        let signature_header = compute_whatsapp_signature_header(app_secret, body);

        assert!(verify_whatsapp_signature(
            app_secret,
            body,
            &signature_header
        ));
    }

    #[test]
    fn whatsapp_signature_unicode_body() {
        let app_secret = "test_secret";
        let body = "Hello 🦀 世界".as_bytes();

        let signature_header = compute_whatsapp_signature_header(app_secret, body);

        assert!(verify_whatsapp_signature(
            app_secret,
            body,
            &signature_header
        ));
    }

    #[test]
    fn whatsapp_signature_json_payload() {
        let app_secret = "my_app_secret_from_meta";
        let body = br#"{"entry":[{"changes":[{"value":{"messages":[{"from":"1234567890","text":{"body":"Hello"}}]}}]}]}"#;

        let signature_header = compute_whatsapp_signature_header(app_secret, body);

        assert!(verify_whatsapp_signature(
            app_secret,
            body,
            &signature_header
        ));
    }

    #[test]
    fn whatsapp_signature_case_sensitive_prefix() {
        let app_secret = "test_secret";
        let body = b"test body";

        let hex_sig = compute_whatsapp_signature_hex(app_secret, body);

        // Wrong case prefix should fail
        let wrong_prefix = format!("SHA256={hex_sig}");
        assert!(!verify_whatsapp_signature(app_secret, body, &wrong_prefix));

        // Correct prefix should pass
        let correct_prefix = format!("sha256={hex_sig}");
        assert!(verify_whatsapp_signature(app_secret, body, &correct_prefix));
    }

    #[test]
    fn whatsapp_signature_truncated_hex() {
        let app_secret = "test_secret";
        let body = b"test body";

        let hex_sig = compute_whatsapp_signature_hex(app_secret, body);
        let truncated = &hex_sig[..32]; // Only half the signature
        let signature_header = format!("sha256={truncated}");

        assert!(!verify_whatsapp_signature(
            app_secret,
            body,
            &signature_header
        ));
    }

    #[test]
    fn whatsapp_signature_extra_bytes() {
        let app_secret = "test_secret";
        let body = b"test body";

        let hex_sig = compute_whatsapp_signature_hex(app_secret, body);
        let extended = format!("{hex_sig}deadbeef");
        let signature_header = format!("sha256={extended}");

        assert!(!verify_whatsapp_signature(
            app_secret,
            body,
            &signature_header
        ));
    }
}
