//! Networkless, disposable rootless Podman execution. Never falls back to host shell.
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Stdio;
use tokio::io::AsyncReadExt;

const SANDBOX_LABEL: &str = "io.raaviclaw.sandbox=true";
const OWNER_PID_LABEL: &str = "io.raaviclaw.owner_pid";
const MAX_OUTPUT_BYTES: usize = 65_536;

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SandboxConfig {
    /// Administrator-built image, pinned by digest; includes approved utilities/wheels.
    pub image: String,
    pub timeout_secs: u64,
    pub memory_mb: u32,
    pub cpus: u32,
}
#[derive(Debug, Serialize)]
pub struct ShellResult {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
    pub truncated: bool,
}
impl SandboxConfig {
    pub fn validate(&self) -> Result<()> {
        let Some((name, hash)) = self.image.split_once("@sha256:") else {
            bail!("Sandbox image must be digest pinned")
        };
        if name.is_empty()
            || name.starts_with('-')
            || self.image.chars().any(char::is_whitespace)
            || hash.len() != 64
            || !hash.bytes().all(|b| b.is_ascii_hexdigit())
            || !(1..=120).contains(&self.timeout_secs)
            || !(64..=1024).contains(&self.memory_mb)
            || !(1..=2).contains(&self.cpus)
        {
            bail!("Invalid sandbox limits or image")
        }
        Ok(())
    }
    /// Remove sandbox containers whose owning `RaaviClaw` process no longer exists.
    /// This is intended for daemon startup after an unclean stop. Active sandboxes
    /// from another live `RaaviClaw` process are left alone.
    pub async fn cleanup_orphans(&self) -> Result<u32> {
        self.validate()?;
        let filter = format!("label={SANDBOX_LABEL}");
        let listing = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            tokio::process::Command::new("podman")
                .args(["ps", "-a", "--filter"])
                .arg(&filter)
                .args([
                    "--format",
                    "{{.Names}}\\t{{.Label \"io.raaviclaw.owner_pid\"}}",
                ])
                .env_clear()
                .env("PATH", "/usr/bin:/bin")
                .env("HOME", std::env::var("HOME")?)
                .env("XDG_RUNTIME_DIR", std::env::var("XDG_RUNTIME_DIR")?)
                .kill_on_drop(true)
                .output(),
        )
        .await??;
        if !listing.status.success() {
            bail!("Unable to enumerate RaaviClaw sandbox containers")
        }

        let mut removed = 0u32;
        for line in String::from_utf8_lossy(&listing.stdout).lines() {
            let mut parts = line.splitn(2, '\t');
            let name = parts.next().unwrap_or("").trim();
            let owner_pid = parts.next().unwrap_or("").trim();
            if !name.starts_with("raaviclaw-") {
                continue;
            }
            if let Ok(pid) = owner_pid.parse::<u32>() {
                if Path::new(&format!("/proc/{pid}")).exists() {
                    continue;
                }
            }
            let status = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                tokio::process::Command::new("podman")
                    .args(["rm", "--force", "--time=0", name])
                    .env_clear()
                    .env("PATH", "/usr/bin:/bin")
                    .env("HOME", std::env::var("HOME")?)
                    .env("XDG_RUNTIME_DIR", std::env::var("XDG_RUNTIME_DIR")?)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .kill_on_drop(true)
                    .status(),
            )
            .await??;
            if !status.success() {
                bail!("Failed to remove orphaned RaaviClaw sandbox {name}")
            }
            removed = removed.saturating_add(1);
        }
        Ok(removed)
    }

    pub async fn execute(&self, workspace: &Path, command: &str) -> Result<ShellResult> {
        let config = self.clone();
        let workspace = workspace.to_owned();
        let command = command.to_owned();
        // Shield cleanup from HTTP cancellation. The internal timeout remains authoritative.
        tokio::spawn(async move {
            static LIMIT: std::sync::OnceLock<tokio::sync::Semaphore> = std::sync::OnceLock::new();
            let _permit = LIMIT
                .get_or_init(|| tokio::sync::Semaphore::new(2))
                .acquire()
                .await?;
            config.execute_inner(&workspace, &command).await
        })
        .await?
    }
    // Keep dispatch and unconditional cleanup together for review.
    #[allow(clippy::too_many_lines)]
    async fn execute_inner(&self, workspace: &Path, command: &str) -> Result<ShellResult> {
        self.validate()?;
        if command.is_empty() || command.len() > 16_000 || command.contains('\0') {
            bail!("Invalid shell command")
        }
        let workspace = workspace.canonicalize()?;
        if workspace == Path::new("/") || workspace.to_string_lossy().contains([':', ',', '\n']) {
            bail!("Invalid task mount")
        }
        let probe = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            tokio::process::Command::new("podman")
                .args(["info", "--format", "{{.Host.Security.Rootless}}"])
                .env_clear()
                .env("PATH", "/usr/bin:/bin")
                .env("HOME", std::env::var("HOME")?)
                .env("XDG_RUNTIME_DIR", std::env::var("XDG_RUNTIME_DIR")?)
                .kill_on_drop(true)
                .output(),
        )
        .await??;
        if !probe.status.success() || String::from_utf8_lossy(&probe.stdout).trim() != "true" {
            bail!("Rootless Podman is required; host execution is forbidden")
        }
        let name = format!("raaviclaw-{}", uuid::Uuid::new_v4());
        let owner_label = format!("{OWNER_PID_LABEL}={}", std::process::id());
        let mut child = tokio::process::Command::new("podman")
            .args(["run", "--pull=never", "--rm", "--name"])
            .arg(&name)
            .args(["--label", SANDBOX_LABEL, "--label"])
            .arg(&owner_label)
            .args([
                "--network=none",
                "--cap-drop=ALL",
                "--security-opt=no-new-privileges",
                "--read-only",
                "--pids-limit=64",
                "--userns=keep-id:uid=1000,gid=1000",
                "--user=1000:1000",
                "--ipc=private",
                "--pid=private",
                "--tmpfs=/tmp:rw,noexec,nosuid,nodev,size=64m",
                "--workdir=/workspace",
                "--tmpfs=/workspace:rw,nosuid,nodev,size=256m,mode=1777",
            ])
            .arg(format!("--memory={}m", self.memory_mb))
            .arg(format!("--memory-swap={}m", self.memory_mb))
            .arg(format!("--cpus={}", self.cpus))
            .arg("--volume")
            .arg(format!("{}:/inputs:ro,nosuid,nodev,Z", workspace.display()))
            .arg(&self.image)
            .args(["sh", "-c", command])
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", std::env::var("HOME")?)
            .env("XDG_RUNTIME_DIR", std::env::var("XDG_RUNTIME_DIR")?)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let out = tokio::spawn(read_bounded(stdout));
        let err = tokio::spawn(read_bounded(stderr));
        let status = tokio::time::timeout(
            std::time::Duration::from_secs(self.timeout_secs),
            child.wait(),
        )
        .await;
        let timed_out = status.is_err();
        // Always remove the named container, including timeout and failed dispatch.
        let cleanup = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            tokio::process::Command::new("podman")
                .args(["rm", "--force", "--time=0", &name])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .kill_on_drop(true)
                .status(),
        )
        .await;
        if timed_out {
            let _ = child.kill().await;
        }
        let (stdout, out_cut) =
            tokio::time::timeout(std::time::Duration::from_secs(5), out).await???;
        let (stderr, err_cut) =
            tokio::time::timeout(std::time::Duration::from_secs(5), err).await???;
        if !matches!(cleanup, Ok(Ok(ref status)) if status.success()) {
            let absent = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                tokio::process::Command::new("podman")
                    .args(["container", "exists", &name])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .kill_on_drop(true)
                    .status(),
            )
            .await;
            if !matches!(absent,Ok(Ok(status)) if status.code()==Some(1)) {
                bail!("Sandbox cleanup failed; administrator intervention required")
            }
        }
        Ok(ShellResult {
            exit_code: status.ok().transpose()?.and_then(|s| s.code()),
            stdout,
            stderr,
            timed_out,
            truncated: out_cut || err_cut,
        })
    }
}
async fn read_bounded(mut stream: impl tokio::io::AsyncRead + Unpin) -> Result<(String, bool)> {
    let mut output = Vec::new();
    let mut chunk = [0; 4096];
    let mut truncated = false;
    loop {
        let count = stream.read(&mut chunk).await?;
        if count == 0 {
            break;
        }
        let remaining = MAX_OUTPUT_BYTES.saturating_sub(output.len());
        output.extend_from_slice(&chunk[..count.min(remaining)]);
        truncated |= count > remaining;
    }
    let mut output = String::from_utf8_lossy(&output).into_owned();
    if output.len() > MAX_OUTPUT_BYTES {
        let mut boundary = MAX_OUTPUT_BYTES;
        while !output.is_char_boundary(boundary) {
            boundary -= 1;
        }
        output.truncate(boundary);
        truncated = true;
    }
    Ok((output, truncated))
}

/// Offline packages only. Network-enabled package commands are never generated.
pub fn package_command(operation: &str, package: Option<&str>) -> Result<String> {
    if let Some(name) = package {
        if name.is_empty()
            || name.len() > 80
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
            || !name.as_bytes()[0].is_ascii_alphanumeric()
        {
            bail!("Invalid reviewed package name")
        }
    }
    match (operation,package) {
        ("package.list",None)=>Ok("python3 -m pip list --format=json".into()),
        ("package.search",Some(name))=>Ok(format!("find /opt/wheels -maxdepth 1 -type f -name '{name}-*.whl' -printf '%f\n'")),
        ("package.install",Some(name))=>Ok(format!("python3 -m pip install --no-index --find-links=/opt/wheels --target=/workspace/packages '{name}' && PYTHONPATH=/workspace/packages python3 -m pip list --path=/workspace/packages --format=json")),
        // Package sets are disposable; removal cannot mutate the image or host.
        ("package.remove",Some(_))=>Ok(r#"rm -rf /workspace/packages && printf '%s\n' '{"removed_disposable_packages":true}'"#.into()),
        _=>bail!("Unsupported package operation or missing package name"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn package_arguments_cannot_inject_shell_or_enable_network() {
        for name in [
            "../x",
            "x;id",
            "$(id)",
            "-option",
            "a b",
            "https://example.invalid/x",
        ] {
            assert!(package_command("package.install", Some(name)).is_err());
        }
        let command = package_command("package.install", Some("approved_tool")).unwrap();
        assert!(command.contains("--no-index"));
        assert!(package_command("package.unknown", Some("tool")).is_err());
    }
    #[test]
    fn only_pinned_bounded_images_are_accepted() {
        let mut c = SandboxConfig {
            image: format!("localhost/raaviclaw@sha256:{}", "a".repeat(64)),
            timeout_secs: 30,
            memory_mb: 256,
            cpus: 1,
        };
        assert!(c.validate().is_ok());
        c.image = "alpine:latest".into();
        assert!(c.validate().is_err());
    }
    #[tokio::test]
    async fn hostile_output_is_bounded_and_drained() {
        let bytes = vec![b'a'; 100_000];
        let (result, truncated) = read_bounded(bytes.as_slice()).await.unwrap();
        assert_eq!(result.len(), 65_536);
        assert!(truncated);
    }
    #[tokio::test]
    async fn invalid_utf8_cannot_expand_past_the_output_limit() {
        let bytes = vec![0xff; MAX_OUTPUT_BYTES];
        let (result, truncated) = read_bounded(bytes.as_slice()).await.unwrap();
        assert!(result.len() <= MAX_OUTPUT_BYTES);
        assert!(truncated);
        assert!(result.chars().all(|character| character == '\u{fffd}'));
        let bytes = vec![b'a'; MAX_OUTPUT_BYTES];
        let (result, truncated) = read_bounded(bytes.as_slice()).await.unwrap();
        assert_eq!(result.len(), MAX_OUTPUT_BYTES);
        assert!(!truncated);
    }
}
