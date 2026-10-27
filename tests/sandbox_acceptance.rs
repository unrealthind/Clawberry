//! Real VM acceptance; explicit opt-in avoids substituting a fake host shell.
use tinyclaw::sandbox::SandboxConfig;
fn config(timeout_secs: u64) -> SandboxConfig {
    SandboxConfig {
        image: std::env::var("RAAVICLAW_SANDBOX_IMAGE").expect("Set a reviewed image digest"),
        timeout_secs,
        memory_mb: 512,
        cpus: 1,
    }
}
#[tokio::test]
#[ignore = "Requires rootless Podman and administrator-built digest-pinned image"]
async fn real_sandbox_is_networkless_readonly_and_bounded() {
    let d = tempfile::TempDir::new().unwrap();
    std::fs::write(d.path().join("input"), "owned evidence").unwrap();
    let script = r#"python3 - <<'PY'
import pathlib,socket,os
assert pathlib.Path('/inputs/input').read_text()=='owned evidence'
try: pathlib.Path('/inputs/input').write_text('tampered')
except OSError: pass
else: raise AssertionError('writable input mount')
assert not pathlib.Path('/etc/tinyclaw/security/root-policy.toml').exists()
assert not pathlib.Path('/home/host-user').exists()
assert not pathlib.Path('/var/run/docker.sock').exists()
assert os.getuid()==1000
assert os.statvfs('/').f_flag & os.ST_RDONLY
assert 'CapEff:\t0000000000000000' in pathlib.Path('/proc/self/status').read_text()
assert 'NoNewPrivs:\t1' in pathlib.Path('/proc/self/status').read_text()
assert int(pathlib.Path('/sys/fs/cgroup/memory.max').read_text()) == 512 * 1024 * 1024
assert int(pathlib.Path('/sys/fs/cgroup/memory.swap.max').read_text()) == 0
assert int(pathlib.Path('/sys/fs/cgroup/pids.max').read_text()) == 64
quota,period=pathlib.Path('/sys/fs/cgroup/cpu.max').read_text().split()
assert int(quota)==int(period)
try: socket.create_connection(('1.1.1.1',443),timeout=1)
except OSError: pass
else: raise AssertionError('network escape')
pathlib.Path('/workspace/output').write_text('scratch')
print('sandbox boundaries passed')
PY
command -v ffmpeg && command -v yt-dlp && command -v pandoc"#;
    let result = config(15).execute(d.path(), script).await.unwrap();
    assert_eq!(result.exit_code, Some(0), "{}", result.stderr);
    assert!(result.stdout.contains("sandbox boundaries passed"));
    assert_eq!(
        std::fs::read_to_string(d.path().join("input")).unwrap(),
        "owned evidence"
    );
    let result = config(1).execute(d.path(), "sleep 30").await.unwrap();
    assert!(result.timed_out);
    let result = config(15)
        .execute(d.path(), "python3 -c 'print(\"a\"*100000)'")
        .await
        .unwrap();
    assert!(result.truncated);
    assert_eq!(result.stdout.len(), 65_536);
}
#[tokio::test]
#[ignore = "Requires rootless Podman and administrator-built digest-pinned image"]
async fn real_media_preview_returns_bounded_artifact() {
    use base64::Engine;
    let d = tempfile::TempDir::new().unwrap();
    // Minimal PPM is a decoder fixture, not an image editing workflow.
    std::fs::write(d.path().join("input"), b"P6\n1 1\n255\n\xff\x00\x00").unwrap();
    let result = config(15)
        .execute(
            d.path(),
            &tinyclaw::media::processing_command("image").unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(result.exit_code, Some(0), "{}", result.stderr);
    let value: serde_json::Value = serde_json::from_str(&result.stdout).unwrap();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(value["artifact_base64"].as_str().unwrap())
        .unwrap();
    assert!(bytes.starts_with(&[0xff, 0xd8]));
    assert!(bytes.len() < 45_000);
}
#[tokio::test]
#[ignore = "Requires rootless Podman and administrator-built digest-pinned image"]
async fn real_video_preview_handles_short_clips() {
    use base64::Engine;
    let d = tempfile::TempDir::new().unwrap();
    let source=config(15).execute(d.path(),"ffmpeg -nostdin -v error -f lavfi -i color=c=red:s=64x64:d=2 -c:v mpeg4 /workspace/clip.mp4 && python3 -c 'import base64;print(base64.b64encode(open(\"/workspace/clip.mp4\",\"rb\").read()).decode())'").await.unwrap();
    assert_eq!(source.exit_code, Some(0), "{}", source.stderr);
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(source.stdout.trim())
        .unwrap();
    std::fs::write(d.path().join("input"), bytes).unwrap();
    let result = config(15)
        .execute(
            d.path(),
            &tinyclaw::media::processing_command("video").unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(result.exit_code, Some(0), "{}", result.stderr);
    let value: serde_json::Value = serde_json::from_str(&result.stdout).unwrap();
    assert!(value["artifact_base64"].as_str().unwrap().len() < 65_000);
}

#[test]
#[ignore = "Requires the installed root-owned host policy, run as the non-root service user"]
fn real_host_policy_loads_and_remains_default_deny() {
    let policy = tinyclaw::security::root::RootPolicy::load(std::path::Path::new(
        "/etc/tinyclaw/security/root-policy.toml",
    ))
    .unwrap();
    let r = tinyclaw::security::ExecutionRequest::new(
        tinyclaw::assistants::AgentId::Janet,
        "task",
        "shell",
        "shell.execute",
        "task",
        serde_json::json!({"command":"true"}),
    );
    assert!(!policy.allows(&r));
}

#[tokio::test]
#[ignore = "Requires rootless Podman and the reviewed offline-wheel fixture image"]
async fn real_packages_install_offline_and_remain_disposable() {
    let mut sandbox = config(30);
    sandbox.image = std::env::var("RAAVICLAW_PACKAGE_TEST_IMAGE")
        .expect("Build the offline-wheel fixture with dev/build-package-fixture.py");
    let directory = tempfile::TempDir::new().unwrap();
    let package = "raaviclaw_acceptance_fixture";
    let search = tinyclaw::sandbox::package_command("package.search", Some(package)).unwrap();
    let install = tinyclaw::sandbox::package_command("package.install", Some(package)).unwrap();
    let remove = tinyclaw::sandbox::package_command("package.remove", Some(package)).unwrap();
    let command = format!(
        "set -eu\n{search}\n{install}\nPYTHONPATH=/workspace/packages python3 -c 'import raaviclaw_acceptance_fixture as fixture; assert fixture.MARKER == \"offline-fixture-installed\"'\n{remove}\ntest ! -e /workspace/packages\n"
    );
    let result = sandbox.execute(directory.path(), &command).await.unwrap();
    assert_eq!(result.exit_code, Some(0), "{}", result.stderr);
    assert!(result
        .stdout
        .lines()
        .any(|line| line == "raaviclaw_acceptance_fixture-0.0.1-py3-none-any.whl"));
    let removed: serde_json::Value =
        serde_json::from_str(result.stdout.lines().last().unwrap()).unwrap();
    assert_eq!(removed["removed_disposable_packages"], true);
    let clean = sandbox
        .execute(directory.path(), "test ! -e /workspace/packages")
        .await
        .unwrap();
    assert_eq!(clean.exit_code, Some(0));
    let missing = sandbox
        .execute(
            directory.path(),
            &tinyclaw::sandbox::package_command(
                "package.install",
                Some("raaviclaw_unbundled_package"),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    assert_ne!(missing.exit_code, Some(0));
}

#[tokio::test]
#[ignore = "Requires rootless Podman and administrator-built digest-pinned image"]
async fn real_orphan_recovery_removes_only_dead_owner_sandboxes() {
    let image = std::env::var("RAAVICLAW_SANDBOX_IMAGE").expect("Set a reviewed image digest");
    let orphan = format!("raaviclaw-orphan-test-{}", uuid::Uuid::new_v4());
    let live = format!("raaviclaw-live-test-{}", uuid::Uuid::new_v4());
    let current_pid = std::process::id().to_string();

    let start = |name: &str, owner_pid: &str| {
        std::process::Command::new("podman")
            .args([
                "run",
                "-d",
                "--pull=never",
                "--network=none",
                "--label",
                "io.raaviclaw.sandbox=true",
                "--label",
            ])
            .arg(format!("io.raaviclaw.owner_pid={owner_pid}"))
            .args(["--name", name])
            .arg(&image)
            .args(["sh", "-c", "sleep 60"])
            .status()
            .expect("podman start")
    };

    assert!(start(&orphan, "4294967294").success());
    assert!(start(&live, &current_pid).success());

    let sandbox = config(15);
    let removed = sandbox.cleanup_orphans().await.unwrap();
    assert!(removed >= 1);

    let orphan_exists = std::process::Command::new("podman")
        .args(["container", "exists", &orphan])
        .status()
        .unwrap();
    assert_eq!(
        orphan_exists.code(),
        Some(1),
        "orphan container should be removed"
    );

    let live_exists = std::process::Command::new("podman")
        .args(["container", "exists", &live])
        .status()
        .unwrap();
    assert!(
        live_exists.success(),
        "live owner container must not be removed"
    );

    let _ = std::process::Command::new("podman")
        .args(["rm", "--force", "--time=0", &live])
        .status();
}
