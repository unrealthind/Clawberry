#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::unnecessary_literal_bound,
    clippy::module_name_repetitions,
    clippy::struct_field_names,
    dead_code
)]

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

// ── Always compiled (tiny tier) ────────────────────────────────
mod agent;
mod assistants;
mod channels;
mod config;
mod health;
mod identity;
mod media;
mod memory;
mod n8n;
mod observability;
mod onboard;
mod providers;
mod runtime;
mod sandbox;
mod security;
mod session;
mod skills;
mod tools;
mod util;

// ── Standard tier (+TUI) ──────────────────────────────────────
#[cfg(feature = "tui-feature")]
mod tui;

// ── Full tier (+gateway, daemon, channels, scheduler, etc.) ───
#[cfg(feature = "daemon-feature")]
mod cron;
#[cfg(feature = "daemon-feature")]
mod daemon;
#[cfg(feature = "daemon-feature")]
mod doctor;
#[cfg(feature = "gateway-feature")]
mod gateway;
#[cfg(feature = "daemon-feature")]
mod heartbeat;
#[cfg(feature = "full")]
mod integrations;
#[cfg(feature = "full")]
mod migration;
#[cfg(feature = "daemon-feature")]
mod service;
#[cfg(feature = "skillforge-feature")]
mod skillforge;
#[cfg(feature = "tunnel-feature")]
mod tunnel;

use config::Config;

/// `RaaviClaw` - Zero overhead. Zero compromise. 100% Rust.
#[derive(Parser, Debug)]
#[command(name = "tinyclaw")]
#[command(version = "0.1.0")]
#[command(about = "RaaviClaw — independent work and personal assistants.", long_about = None)]
struct Cli {
    /// Configuration directory (defaults to ~/.tinyclaw)
    #[arg(long, global = true)]
    config_dir: Option<std::path::PathBuf>,
    #[command(subcommand)]
    command: Commands,
}

#[cfg(feature = "daemon-feature")]
#[derive(Subcommand, Debug)]
enum ServiceCommands {
    /// Install daemon service unit for auto-start and restart
    Install,
    /// Start daemon service
    Start,
    /// Stop daemon service
    Stop,
    /// Check daemon service status
    Status,
    /// Uninstall daemon service unit
    Uninstall,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Host-administrator credential registration; value entered with echo disabled
    CredentialRegister {
        name: String,
        #[arg(long, value_delimiter=',', value_parser=["janet","harley"])]
        agents: Vec<String>,
        #[arg(long, value_delimiter = ',')]
        capabilities: Vec<String>,
    },
    /// List permanent assistants and their isolated storage
    Agents,

    /// Perform an owned internal operation using a JSON request
    Work {
        #[arg(value_parser = ["janet", "harley"])]
        agent: String,
        /// JSON operation (or use --request-file)
        #[arg(long, conflicts_with = "request_file")]
        request: Option<String>,
        #[arg(long, conflicts_with = "request")]
        request_file: Option<std::path::PathBuf>,
    },

    /// List durable model usage and cost records
    Usage {
        #[arg(value_parser = ["janet", "harley"])]
        agent: String,
    },

    /// Run a scoped temporary worker from a JSON specification and wait for its archive
    Worker {
        #[arg(value_parser = ["janet", "harley"])]
        agent: String,
        #[arg(long)]
        spec: std::path::PathBuf,
    },

    /// Chat with Janet or Harley (conversation only; execution tools disabled)
    Chat {
        /// Permanent assistant identity
        #[arg(value_parser = ["janet", "harley"])]
        agent: String,
        /// Send a single message instead of entering interactive chat
        #[arg(short, long)]
        message: Option<String>,
        /// Runtime state directory (defaults to workspace/assistants)
        #[arg(long)]
        state_dir: Option<std::path::PathBuf>,
        /// Optional owned Focus Thread
        #[arg(long)]
        thread: Option<String>,
        /// Runtime reasoning profile
        #[arg(long, default_value="normal",value_parser=["fast","normal","deep"])]
        profile: String,
    },
    /// Initialize your workspace and configuration
    Onboard {
        /// Run the full interactive wizard (default is quick setup)
        #[arg(long)]
        interactive: bool,

        /// Reconfigure channels only (fast repair flow)
        #[arg(long)]
        channels_only: bool,

        /// API key (used in quick mode, ignored with --interactive)
        #[arg(long)]
        api_key: Option<String>,

        /// Provider name (used in quick mode, default: openrouter)
        #[arg(long)]
        provider: Option<String>,

        /// Memory backend (sqlite, markdown, none) - used in quick mode, default: sqlite
        #[arg(long)]
        memory: Option<String>,
    },

    /// Launch the TUI (terminal UI) chat interface
    #[cfg(feature = "tui-feature")]
    Tui {
        /// Provider to use (openrouter, anthropic, openai, ollama)
        #[arg(short, long)]
        provider: Option<String>,

        /// Model to use
        #[arg(long)]
        model: Option<String>,

        /// Temperature (0.0 - 2.0)
        #[arg(short, long, default_value = "0.7")]
        temperature: f64,
    },

    /// Start the AI agent loop
    Agent {
        /// Single message mode (don't enter interactive mode)
        #[arg(short, long)]
        message: Option<String>,

        /// Provider to use (openrouter, anthropic, openai)
        #[arg(short, long)]
        provider: Option<String>,

        /// Model to use
        #[arg(long)]
        model: Option<String>,

        /// Temperature (0.0 - 2.0)
        #[arg(short, long, default_value = "0.7")]
        temperature: f64,
    },

    /// Start the gateway server (webhooks, websockets)
    #[cfg(feature = "gateway-feature")]
    Gateway {
        /// Port to listen on (use 0 for random available port)
        #[arg(short, long, default_value = "8080")]
        port: u16,

        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Token-free assistant UI (direct loopback only; no tunnels/proxy trust)
        #[arg(long)]
        local_ui: bool,

        /// Print a fresh development access key; paired loopback gateway only
        #[arg(long, conflicts_with = "local_ui")]
        dev_access: bool,
    },

    /// Start long-running autonomous runtime (gateway + channels + heartbeat + scheduler)
    #[cfg(feature = "daemon-feature")]
    Daemon {
        /// Port to listen on (use 0 for random available port)
        #[arg(short, long, default_value = "8080")]
        port: u16,

        /// Host to bind to
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Token-free assistant UI (direct loopback only; no tunnels/proxy trust)
        #[arg(long)]
        local_ui: bool,
    },

    /// Manage OS service lifecycle (launchd/systemd user service)
    #[cfg(feature = "daemon-feature")]
    Service {
        #[command(subcommand)]
        service_command: ServiceCommands,
    },

    /// Run diagnostics for daemon/scheduler/channel freshness
    #[cfg(feature = "daemon-feature")]
    Doctor,

    /// Show system status (full details)
    Status,

    /// Configure and manage scheduled tasks
    #[cfg(feature = "daemon-feature")]
    Cron {
        #[command(subcommand)]
        cron_command: CronCommands,
    },

    /// Manage channels (telegram, discord, slack)
    #[cfg(feature = "channels-feature")]
    Channel {
        #[command(subcommand)]
        channel_command: ChannelCommands,
    },

    /// Browse 50+ integrations
    #[cfg(feature = "full")]
    Integrations {
        #[command(subcommand)]
        integration_command: IntegrationCommands,
    },

    /// Manage skills (user-defined capabilities)
    Skills {
        #[command(subcommand)]
        skill_command: SkillCommands,
    },

    /// Migrate data from other agent runtimes
    #[cfg(feature = "full")]
    Migrate {
        #[command(subcommand)]
        migrate_command: MigrateCommands,
    },
}

#[cfg(feature = "full")]
#[derive(Subcommand, Debug)]
enum MigrateCommands {
    /// Import memory from an `OpenClaw` workspace into this `RaaviClaw` workspace
    Openclaw {
        /// Optional path to `OpenClaw` workspace (defaults to ~/.openclaw/workspace)
        #[arg(long)]
        source: Option<std::path::PathBuf>,

        /// Validate and preview migration without writing any data
        #[arg(long)]
        dry_run: bool,
    },
}

#[cfg(feature = "daemon-feature")]
#[derive(Subcommand, Debug)]
enum CronCommands {
    /// List all scheduled tasks
    List,
    /// Add a new scheduled task
    Add {
        /// Cron expression
        expression: String,
        /// Command to run
        command: String,
    },
    /// Remove a scheduled task
    Remove {
        /// Task ID
        id: String,
    },
}

#[cfg(feature = "channels-feature")]
#[derive(Subcommand, Debug)]
enum ChannelCommands {
    /// List configured channels
    List,
    /// Start all configured channels (Telegram, Discord, Slack)
    Start,
    /// Run health checks for configured channels
    Doctor,
    /// Add a new channel
    Add {
        /// Channel type
        channel_type: String,
        /// Configuration JSON
        config: String,
    },
    /// Remove a channel
    Remove {
        /// Channel name
        name: String,
    },
}

#[derive(Subcommand, Debug)]
enum SkillCommands {
    /// List installed skills
    List,
    /// Install a skill from a GitHub URL or local path
    Install {
        /// GitHub URL or local path
        source: String,
    },
    /// Remove an installed skill
    Remove {
        /// Skill name
        name: String,
    },
}

#[cfg(feature = "full")]
#[derive(Subcommand, Debug)]
enum IntegrationCommands {
    /// Show details about a specific integration
    Info {
        /// Integration name
        name: String,
    },
}

#[tokio::main]
#[allow(clippy::too_many_lines)]
async fn main() -> Result<()> {
    // Install default crypto provider for Rustls TLS.
    if let Err(e) = rustls::crypto::ring::default_provider().install_default() {
        eprintln!("Warning: Failed to install default crypto provider: {e:?}");
    }

    let cli = Cli::parse();

    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // Onboard runs quick setup by default, or the interactive wizard with --interactive
    if let Commands::Onboard {
        interactive,
        channels_only,
        api_key,
        provider,
        memory,
    } = &cli.command
    {
        if cli.config_dir.is_some() {
            bail!("--config-dir is supported for runtime commands; onboarding currently uses ~/.tinyclaw. Edit the selected config.toml directly instead.");
        }
        if *interactive && *channels_only {
            bail!("Use either --interactive or --channels-only, not both");
        }
        if *channels_only && (api_key.is_some() || provider.is_some() || memory.is_some()) {
            bail!("--channels-only does not accept --api-key, --provider, or --memory");
        }

        let config = if *channels_only {
            onboard::run_channels_repair_wizard()?
        } else if *interactive {
            onboard::run_wizard()?
        } else {
            onboard::run_quick_setup(api_key.as_deref(), provider.as_deref(), memory.as_deref())?
        };
        // Auto-start channels if user said yes during wizard
        #[cfg(feature = "channels-feature")]
        if std::env::var("ZEROCLAW_AUTOSTART_CHANNELS").as_deref() == Ok("1") {
            channels::start_channels(config).await?;
        }
        return Ok(());
    }

    // All other commands need config loaded first
    let config = if let Some(directory) = &cli.config_dir {
        Config::load_or_init_in(directory)?
    } else {
        Config::load_or_init()?
    };

    match cli.command {
        Commands::Onboard { .. } => unreachable!(),

        Commands::Agents => {
            let host = assistants::AgentHost::open(&config.workspace_dir.join("assistants"))?;
            println!("{}", serde_json::to_string_pretty(host.records())?);
            Ok(())
        }
        Commands::CredentialRegister {
            name,
            agents,
            capabilities,
        } => {
            let root = config.workspace_dir.join("assistants");
            let _host = assistants::AgentHost::open(&root)?;
            let directory = root.join("credentials");
            if !directory.exists() {
                std::fs::create_dir(&directory)?;
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))?;
            }
            let value = dialoguer::Password::new()
                .with_prompt("Credential value (hidden; never put it in command arguments)")
                .interact()?;
            let agents = agents
                .iter()
                .map(|s| assistants::AgentId::parse(s))
                .collect::<Result<Vec<_>>>()?;
            security::SecretBroker::open(&directory, true)?.register(
                &name,
                agents,
                capabilities,
                &value,
            )?;
            println!("Registered encrypted credential reference: {name}");
            Ok(())
        }

        Commands::Work {
            agent,
            request,
            request_file,
        } => {
            let host = assistants::AgentHost::open(&config.workspace_dir.join("assistants"))?;
            let owner = assistants::AgentId::parse(&agent)?;
            let json = if let Some(path) = request_file {
                std::fs::read_to_string(path)?
            } else {
                request.ok_or_else(|| anyhow::anyhow!("Provide --request or --request-file"))?
            };
            let result = host
                .operate(&config, owner, serde_json::from_str(&json)?)
                .await?;
            host.wait_active_workers(owner).await?;
            println!("{}", serde_json::to_string_pretty(&result)?);
            Ok(())
        }

        Commands::Usage { agent } => {
            let host = assistants::AgentHost::open(&config.workspace_dir.join("assistants"))?;
            println!(
                "{}",
                serde_json::to_string_pretty(&host.usage(assistants::AgentId::parse(&agent)?)?)?
            );
            Ok(())
        }

        Commands::Worker { agent, spec } => {
            let host = assistants::AgentHost::open(&config.workspace_dir.join("assistants"))?;
            let owner = assistants::AgentId::parse(&agent)?;
            let worker = host.spawn_worker(
                &config,
                owner,
                serde_json::from_str(&std::fs::read_to_string(spec)?)?,
            )?;
            println!(
                "{}",
                serde_json::to_string_pretty(&host.wait_worker(owner, &worker.id).await?)?
            );
            Ok(())
        }

        Commands::Chat {
            agent,
            message,
            state_dir,
            thread,
            profile,
        } => {
            let id = assistants::AgentId::parse(&agent)?;
            let host = assistants::AgentHost::open(
                &state_dir.unwrap_or_else(|| config.workspace_dir.join("assistants")),
            )?;
            let profile = assistants::models::Profile::parse(&profile)?;
            if let Some(message) = message {
                println!(
                    "{}",
                    host.chat_scoped(&config, id, thread.as_deref(), None, profile, &message)
                        .await?
                );
                host.wait_active_workers(id).await?;
            } else {
                use std::io::Write;
                println!(
                    "{} main chat. Type /quit to exit. Execution tools are disabled.",
                    host.record(id).definition.name
                );
                loop {
                    print!("> ");
                    std::io::stdout().flush()?;
                    let mut message = String::new();
                    if std::io::stdin().read_line(&mut message)? == 0 || message.trim() == "/quit" {
                        break;
                    }
                    match host
                        .chat_scoped(
                            &config,
                            id,
                            thread.as_deref(),
                            None,
                            profile,
                            message.trim(),
                        )
                        .await
                    {
                        Ok(response) => println!("{response}"),
                        Err(error) => eprintln!("{error:#}"),
                    }
                }
                host.wait_active_workers(id).await?;
            }
            Ok(())
        }

        #[cfg(feature = "tui-feature")]
        Commands::Tui {
            provider,
            model,
            temperature,
        } => tui::run(config, provider, model, temperature).await,

        Commands::Agent {
            message,
            provider,
            model,
            temperature,
        } => agent::run(config, message, provider, model, temperature).await,

        #[cfg(feature = "gateway-feature")]
        Commands::Gateway {
            port,
            host,
            local_ui,
            dev_access,
        } => {
            let mut config = config;
            if dev_access {
                use sha2::Digest;
                if !host.parse::<std::net::IpAddr>()?.is_loopback() {
                    anyhow::bail!("Development access requires a loopback gateway; expose the paired GUI instead");
                }
                if config.tunnel.provider != "none" {
                    anyhow::bail!("Development access requires tunnels to be disabled");
                }
                let token = format!(
                    "zc_{}{}",
                    uuid::Uuid::new_v4().simple(),
                    uuid::Uuid::new_v4().simple()
                );
                config.gateway.local_ui = false;
                config.gateway.require_pairing = true;
                config
                    .gateway
                    .paired_tokens
                    .push(hex::encode(sha2::Sha256::digest(token.as_bytes())));
                println!("\nDEVELOPMENT ACCESS KEY (this gateway run):\n{token}\n");
            } else {
                config.gateway.local_ui |= local_ui;
            }
            if port == 0 {
                info!("Starting RaaviClaw Gateway on {host} (random port)");
            } else {
                info!("Starting RaaviClaw Gateway on {host}:{port}");
            }
            gateway::run_gateway(&host, port, config).await
        }

        #[cfg(feature = "daemon-feature")]
        Commands::Daemon {
            port,
            host,
            local_ui,
        } => {
            let mut config = config;
            config.gateway.local_ui |= local_ui;
            if port == 0 {
                info!("Starting RaaviClaw Daemon on {host} (random port)");
            } else {
                info!("Starting RaaviClaw Daemon on {host}:{port}");
            }
            daemon::run(config, host, port).await
        }

        Commands::Status => {
            println!("RaaviClaw Status");
            println!();
            println!("Version:     {}", env!("CARGO_PKG_VERSION"));
            println!("Workspace:   {}", config.workspace_dir.display());
            println!("Config:      {}", config.config_path.display());
            println!();
            println!(
                "Provider:      {}",
                config.default_provider.as_deref().unwrap_or("openrouter")
            );
            println!(
                "Model:         {}",
                config.default_model.as_deref().unwrap_or("(default)")
            );
            println!("Observability: {}", config.observability.backend);
            println!("Autonomy:      {:?}", config.autonomy.level);
            println!("Runtime:       {}", config.runtime.kind);
            println!(
                "Heartbeat:     {}",
                if config.heartbeat.enabled {
                    format!("every {}min", config.heartbeat.interval_minutes)
                } else {
                    "disabled".into()
                }
            );
            println!(
                "Memory:        {} (auto-save: {})",
                config.memory.backend,
                if config.memory.auto_save { "on" } else { "off" }
            );

            #[cfg(feature = "tiny")]
            {
                let tier = if cfg!(feature = "full") {
                    "full"
                } else if cfg!(feature = "standard") {
                    "standard"
                } else {
                    "tiny"
                };
                println!("Build tier:    {tier}");
            }

            Ok(())
        }

        #[cfg(feature = "daemon-feature")]
        Commands::Cron { cron_command } => cron::handle_command(cron_command, &config),

        #[cfg(feature = "daemon-feature")]
        Commands::Service { service_command } => service::handle_command(&service_command, &config),

        #[cfg(feature = "daemon-feature")]
        Commands::Doctor => doctor::run(&config),

        #[cfg(feature = "channels-feature")]
        Commands::Channel { channel_command } => match channel_command {
            ChannelCommands::Start => channels::start_channels(config).await,
            ChannelCommands::Doctor => channels::doctor_channels(config).await,
            other => channels::handle_command(other, &config),
        },

        #[cfg(feature = "full")]
        Commands::Integrations {
            integration_command,
        } => integrations::handle_command(integration_command, &config),

        Commands::Skills { skill_command } => {
            skills::handle_command(skill_command, &config.workspace_dir)
        }

        #[cfg(feature = "full")]
        Commands::Migrate { migrate_command } => {
            migration::handle_command(migrate_command, &config).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_has_no_flag_conflicts() {
        Cli::command().debug_assert();
    }
}
