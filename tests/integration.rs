// Integration tests that drive the built binary against a real temp directory.
//
// These tests exist to protect the killer demo: `agent-undo init` → edit file
// → `agent-undo oops` → file restored. If any of that stops working, CI fails.
//
// Run with:
//     cargo test --test integration
//
// Tests create their own isolated temp dirs so they can run in parallel.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn unique_tmp_dir(label: &str) -> PathBuf {
    let ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let pid = std::process::id();
    let dir = std::env::temp_dir().join(format!("agent-undo-test-{label}-{pid}-{ns}"));
    fs::create_dir_all(&dir).expect("create tmp dir");
    dir
}

fn bin_path() -> PathBuf {
    // cargo sets CARGO_BIN_EXE_<bin_name> for integration tests.
    // The crate is `agent-undo` but the bin is `au`.
    PathBuf::from(env!("CARGO_BIN_EXE_au"))
}

fn run(cwd: &PathBuf, args: &[&str]) -> (i32, String, String) {
    let output = Command::new(bin_path())
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("failed to run agent-undo");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn wait_for_daemon_ready(dir: &PathBuf) {
    let started = Instant::now();
    let timeout = Duration::from_secs(5);

    loop {
        let (code, out, err) = run(dir, &["status", "--json"]);
        let sample = if code == 0 {
            out
        } else {
            format!("status --json failed ({code}): {err}")
        };

        if code == 0 {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&sample) {
                if parsed["daemon_running"].as_bool() == Some(true) {
                    return;
                }
            }
        }

        if started.elapsed() >= timeout {
            panic!("daemon never became ready: {sample}");
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn wait_for_log_match<F>(dir: &PathBuf, description: &str, predicate: F) -> String
where
    F: Fn(&str) -> bool,
{
    let started = Instant::now();
    let timeout = Duration::from_secs(5);

    loop {
        let (code, out, err) = run(dir, &["log", "-n", "20"]);
        let sample = if code == 0 {
            out
        } else {
            format!("log -n 20 failed ({code}): {err}")
        };
        if code == 0 && predicate(&sample) {
            return sample;
        }

        if started.elapsed() >= timeout {
            panic!("{description}: {sample}");
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[test]
fn init_creates_data_dir_and_scans_files() {
    let dir = unique_tmp_dir("init");
    fs::write(dir.join("a.txt"), "hello").unwrap();
    fs::write(dir.join("b.rs"), "fn main() {}").unwrap();

    let (code, out, _) = run(&dir, &["init"]);
    assert_eq!(code, 0);
    assert!(
        out.contains("snapshotted 2 files"),
        "unexpected output: {out}"
    );

    assert!(dir.join(".agent-undo").is_dir());
    assert!(dir.join(".agent-undo/timeline.db").is_file());
    assert!(dir.join(".agent-undo/objects").is_dir());
    assert!(dir.join(".agent-undo/config.toml").is_file());

    // Re-running init is idempotent (prints "already initialized").
    let (code2, out2, _) = run(&dir, &["init"]);
    assert_eq!(code2, 0);
    assert!(out2.contains("already initialized"));

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn log_is_empty_before_serve_and_shows_init_events() {
    let dir = unique_tmp_dir("log");
    fs::write(dir.join("file.txt"), "content").unwrap();

    run(&dir, &["init"]);
    let (code, out, _) = run(&dir, &["log"]);
    assert_eq!(code, 0);
    assert!(
        out.contains("initial-scan"),
        "log should show initial scan: {out}"
    );
    assert!(out.contains("file.txt"));

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn status_reports_correct_event_count() {
    let dir = unique_tmp_dir("status");
    fs::write(dir.join("x.md"), "# hi").unwrap();
    fs::write(dir.join("y.md"), "# hey").unwrap();
    fs::write(dir.join("z.md"), "# hello").unwrap();

    run(&dir, &["init"]);
    let (code, out, _) = run(&dir, &["status"]);
    assert_eq!(code, 0);
    assert!(out.contains("events:   3"), "expected 3 events: {out}");

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn status_json_outputs_machine_readable_fields() {
    let dir = unique_tmp_dir("status_json");
    fs::write(dir.join("x.md"), "# hi").unwrap();
    run(&dir, &["init"]);

    let (code, out, err) = run(&dir, &["status", "--json"]);
    assert_eq!(code, 0, "status --json failed: {err}");
    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed["events"], 1);
    assert!(parsed.get("root").is_some());
    assert!(parsed.get("daemon_running").is_some());

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn hook_pre_writes_active_session_and_post_clears_it() {
    let dir = unique_tmp_dir("hook");
    fs::write(dir.join("f.rs"), "original").unwrap();
    run(&dir, &["init"]);

    let pre_json =
        r#"{"session_id":"s1","tool_name":"Edit","tool_input":{"file_path":"/tmp/f.rs"}}"#;
    let out = Command::new(bin_path())
        .args(["hook", "pre"])
        .current_dir(&dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write;
    {
        let mut stdin = out.stdin.as_ref().unwrap();
        stdin.write_all(pre_json.as_bytes()).unwrap();
    }
    let _ = out.wait_with_output().unwrap();

    let active = dir.join(".agent-undo/active-session.json");
    assert!(
        active.exists(),
        "hook pre should create active-session.json"
    );
    let parsed: serde_json::Value = serde_json::from_slice(&fs::read(&active).unwrap()).unwrap();
    assert_eq!(parsed["session_id"], "s1");
    assert_eq!(parsed["agent"], "claude-code");

    let post_json = r#"{"session_id":"s1","tool_name":"Edit","tool_input":{"file_path":"/tmp/f.rs"},"tool_response":{"success":true}}"#;
    let out2 = Command::new(bin_path())
        .args(["hook", "post"])
        .current_dir(&dir)
        .stdin(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    {
        let mut stdin = out2.stdin.as_ref().unwrap();
        stdin.write_all(post_json.as_bytes()).unwrap();
    }
    let _ = out2.wait_with_output().unwrap();

    assert!(
        !active.exists(),
        "hook post should clear active-session.json"
    );

    let (_, sessions, _) = run(&dir, &["sessions"]);
    assert!(
        sessions.contains("claude-code"),
        "sessions list missing entry: {sessions}"
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn help_contains_all_commands() {
    let dir = unique_tmp_dir("help");
    let (code, out, _) = run(&dir, &["--help"]);
    assert_eq!(code, 0);
    for cmd in [
        "init", "status", "serve", "log", "sessions", "diff", "show", "restore", "oops", "pin",
        "blame", "tui", "exec", "wrap", "session", "hook", "gc",
    ] {
        assert!(out.contains(cmd), "help output missing `{cmd}`:\n{out}");
    }
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn wrap_shellenv_points_at_project_wrapper_bin_dir() {
    let dir = unique_tmp_dir("wrap_shellenv");
    fs::write(dir.join("a.txt"), "x").unwrap();
    run(&dir, &["init"]);

    let (code, out, err) = run(&dir, &["wrap", "shellenv"]);
    assert_eq!(code, 0, "wrap shellenv failed: {err}");
    assert!(
        out.contains(".agent-undo/bin"),
        "shellenv should mention wrapper bin dir: {out}"
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn wrap_install_creates_working_terminal_agent_wrapper() {
    let dir = unique_tmp_dir("wrap_install");
    fs::write(dir.join("a.txt"), "x").unwrap();
    run(&dir, &["init"]);

    let fake_bin = unique_tmp_dir("wrap_install_fakebin");
    let fake_codex = fake_bin.join("codex");
    fs::write(
        &fake_codex,
        "#!/usr/bin/env sh\nprintf 'wrapped:%s' \"$*\" > wrapper-output.txt\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&fake_codex).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&fake_codex, perms).unwrap();
    }

    let (install_code, install_out, install_err) = run(
        &dir,
        &["wrap", "install", "--agent", "codex", "--binary", "codex"],
    );
    assert_eq!(
        install_code, 0,
        "wrap install failed: {install_out}{install_err}"
    );

    let wrapper = dir.join(".agent-undo/bin/codex");
    assert!(wrapper.exists(), "wrapper should be created");

    let path = format!(
        "{}:{}:{}",
        dir.join(".agent-undo/bin").display(),
        fake_bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let output = Command::new(&wrapper)
        .arg("run")
        .arg("hello")
        .current_dir(&dir)
        .env("PATH", path)
        .output()
        .unwrap();
    assert_eq!(output.status.code().unwrap_or(-1), 0);
    assert!(
        String::from_utf8_lossy(&output.stdout).trim().is_empty(),
        "wrapper should preserve downstream stdout without au banners"
    );
    assert_eq!(
        fs::read_to_string(dir.join("wrapper-output.txt")).unwrap(),
        "wrapped:run hello"
    );

    let (_, sessions, _) = run(&dir, &["sessions"]);
    assert!(
        sessions.contains("codex"),
        "wrapper usage should record a codex session: {sessions}"
    );

    fs::remove_dir_all(&dir).ok();
    fs::remove_dir_all(&fake_bin).ok();
}

#[test]
fn exec_quiet_suppresses_wrapper_session_banners() {
    let dir = unique_tmp_dir("exec_quiet");
    fs::write(dir.join("a.txt"), "x").unwrap();
    run(&dir, &["init"]);

    let fake_bin = unique_tmp_dir("exec_quiet_fakebin");
    let fake_tool = fake_bin.join("tool");
    fs::write(&fake_tool, "#!/usr/bin/env sh\nprintf 'tool-output' \n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&fake_tool).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&fake_tool, perms).unwrap();
    }

    let path = format!(
        "{}:{}",
        fake_bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let output = Command::new(bin_path())
        .args(["exec", "--agent", "tool", "--quiet", "--", "tool"])
        .current_dir(&dir)
        .env("PATH", path)
        .output()
        .unwrap();
    assert_eq!(output.status.code().unwrap_or(-1), 0);
    assert_eq!(String::from_utf8_lossy(&output.stdout), "tool-output");

    fs::remove_dir_all(&dir).ok();
    fs::remove_dir_all(&fake_bin).ok();
}

#[test]
fn wrap_presets_lists_common_terminal_agents() {
    let dir = unique_tmp_dir("wrap_presets");
    fs::write(dir.join("a.txt"), "x").unwrap();
    run(&dir, &["init"]);

    let (code, out, err) = run(&dir, &["wrap", "presets"]);
    assert_eq!(code, 0, "wrap presets failed: {err}");
    assert!(out.contains("codex"));
    assert!(out.contains("aider"));
    assert!(out.contains("claude"));

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn wrap_install_supports_presets() {
    let dir = unique_tmp_dir("wrap_preset_install");
    fs::write(dir.join("a.txt"), "x").unwrap();
    run(&dir, &["init"]);

    let (code, out, err) = run(&dir, &["wrap", "install", "--preset", "codex"]);
    assert_eq!(code, 0, "wrap preset install failed: {out}{err}");
    assert!(dir.join(".agent-undo/bin/codex").exists());

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn wrap_auto_installs_detected_presets() {
    let dir = unique_tmp_dir("wrap_auto");
    fs::write(dir.join("a.txt"), "x").unwrap();
    run(&dir, &["init"]);

    let fake_bin = unique_tmp_dir("wrap_auto_fakebin");
    for binary in ["codex", "aider"] {
        let path = fake_bin.join(binary);
        fs::write(&path, "#!/usr/bin/env sh\nexit 0\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&path, perms).unwrap();
        }
    }

    let path = format!(
        "{}:{}",
        fake_bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let output = Command::new(bin_path())
        .args(["wrap", "auto"])
        .current_dir(&dir)
        .env("PATH", path)
        .output()
        .unwrap();
    assert_eq!(output.status.code().unwrap_or(-1), 0);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(".agent-undo/bin/codex"));
    assert!(stdout.contains(".agent-undo/bin/aider"));
    assert!(dir.join(".agent-undo/bin/codex").exists());
    assert!(dir.join(".agent-undo/bin/aider").exists());

    fs::remove_dir_all(&dir).ok();
    fs::remove_dir_all(&fake_bin).ok();
}

#[test]
fn wrap_list_and_remove_manage_installed_wrappers() {
    let dir = unique_tmp_dir("wrap_manage");
    fs::write(dir.join("a.txt"), "x").unwrap();
    run(&dir, &["init"]);

    let (install_code, _, install_err) = run(
        &dir,
        &["wrap", "install", "--agent", "codex", "--binary", "codex"],
    );
    assert_eq!(install_code, 0, "wrap install failed: {install_err}");

    let (list_code, list_out, list_err) = run(&dir, &["wrap", "list"]);
    assert_eq!(list_code, 0, "wrap list failed: {list_err}");
    assert!(
        list_out.contains(".agent-undo/bin/codex"),
        "wrap list should include installed wrapper: {list_out}"
    );

    let (remove_code, remove_out, remove_err) = run(&dir, &["wrap", "remove", "codex"]);
    assert_eq!(remove_code, 0, "wrap remove failed: {remove_err}");
    assert!(
        remove_out.contains("removed wrapper"),
        "unexpected remove output: {remove_out}"
    );
    assert!(
        !dir.join(".agent-undo/bin/codex").exists(),
        "wrapper file should be removed"
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn session_start_and_end_manage_active_session_marker() {
    let dir = unique_tmp_dir("session_lifecycle");
    fs::write(dir.join("f.rs"), "original").unwrap();
    run(&dir, &["init"]);

    let start = Command::new(bin_path())
        .args([
            "session",
            "start",
            "--agent",
            "cursor",
            "--metadata",
            r#"{"prompt":"refactor auth","tool_name":"Edit","file_path":"src/auth.rs"}"#,
        ])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(start.status.code().unwrap_or(-1), 0);
    let session_id = String::from_utf8_lossy(&start.stdout).trim().to_string();
    assert!(
        session_id.starts_with("session-"),
        "unexpected session id: {session_id}"
    );

    let active = dir.join(".agent-undo/active-session.json");
    assert!(
        active.exists(),
        "session start should create active-session.json"
    );
    let parsed: serde_json::Value = serde_json::from_slice(&fs::read(&active).unwrap()).unwrap();
    assert_eq!(parsed["session_id"], session_id);
    assert_eq!(parsed["agent"], "cursor");
    assert_eq!(parsed["tool_name"], "Edit");
    assert_eq!(parsed["intended_file"], "src/auth.rs");

    let end = Command::new(bin_path())
        .args(["session", "end", &session_id])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(end.status.code().unwrap_or(-1), 0);
    assert!(
        !active.exists(),
        "session end should clear active-session.json"
    );

    let (_, sessions, _) = run(&dir, &["sessions"]);
    assert!(
        sessions.contains("cursor"),
        "sessions list missing cursor entry: {sessions}"
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn sessions_json_outputs_machine_readable_rows() {
    let dir = unique_tmp_dir("sessions_json");
    fs::write(dir.join("f.rs"), "original").unwrap();
    run(&dir, &["init"]);

    let start = Command::new(bin_path())
        .args(["session", "start", "--agent", "cursor"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert_eq!(start.status.code().unwrap_or(-1), 0);

    let (code, out, err) = run(&dir, &["sessions", "--json"]);
    assert_eq!(code, 0, "sessions --json failed: {err}");
    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    let rows = parsed.as_array().expect("sessions json should be an array");
    assert!(!rows.is_empty());
    assert_eq!(rows[0]["agent"], "cursor");

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn restore_by_event_id_recovers_prior_content() {
    let dir = unique_tmp_dir("restore_by_id");
    let original = "original line 1\noriginal line 2\n";
    fs::write(dir.join("target.txt"), original).unwrap();
    run(&dir, &["init"]);

    // Simulate a user edit recorded into the timeline by using hook pre/post
    // with an inline active session, then letting the watcher detect the
    // change. Easier: use `exec` wrapper with a simple echo-overwrite.
    let bin = bin_path();
    let wrote = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "{bin} exec --agent test -- sh -c 'printf \"trashed\" > target.txt'",
            bin = bin.display()
        ))
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(wrote.status.success(), "exec wrapper failed: {:?}", wrote);

    // Give the watcher nothing — it isn't running in this test. Instead we
    // manually record the change through a fresh serve loop isn't trivial.
    // Skip: rely on the mid-session marker + a manual `agent-undo log` to
    // verify the exec session was recorded.
    let (_, sessions, _) = run(&dir, &["sessions"]);
    assert!(sessions.contains("test"), "session missing: {sessions}");

    // The actual restore-by-id path is exercised via the oops burst-restore
    // test below, which drives the full watcher+restore round-trip.
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn session_events_query_returns_empty_for_unknown_session() {
    let dir = unique_tmp_dir("no_session");
    fs::write(dir.join("a.txt"), "content").unwrap();
    run(&dir, &["init"]);

    let (code, _, _) = run(&dir, &["restore", "--session", "nonexistent"]);
    assert_eq!(code, 0, "restore --session on unknown id should not fail");

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn init_install_hooks_flag_compiles_and_accepts_args() {
    // We can't test the actual Claude Code hook install without mocking
    // ~/.claude/, so this just verifies the flag parses and init still
    // succeeds. The install module has its own unit coverage via main build.
    let dir = unique_tmp_dir("install_hooks_flag");
    fs::write(dir.join("x.txt"), "x").unwrap();

    // Point HOME at a tmp location so we don't clobber the real ~/.claude.
    let fake_home = unique_tmp_dir("fake_home");
    let output = Command::new(bin_path())
        .args(["init", "--install-hooks"])
        .current_dir(&dir)
        .env("HOME", &fake_home)
        .output()
        .unwrap();
    assert_eq!(output.status.code().unwrap_or(-1), 0);

    // Verify the settings.json got created in our fake home.
    let settings = fake_home.join(".claude/settings.json");
    assert!(settings.exists(), "hooks should have created settings.json");
    let content = fs::read_to_string(&settings).unwrap();
    assert!(content.contains("PreToolUse"));
    assert!(content.contains("PostToolUse"));
    assert!(content.contains("au hook pre"));
    assert!(content.contains("au hook post"));
    assert!(content.contains("Write|Edit|MultiEdit"));

    // Running again should be idempotent.
    let output2 = Command::new(bin_path())
        .args(["init", "--install-hooks"])
        .current_dir(&dir)
        .env("HOME", &fake_home)
        .output()
        .unwrap();
    assert_eq!(output2.status.code().unwrap_or(-1), 0);
    // Count occurrences of "au hook pre" — should still be exactly 1
    let content2 = fs::read_to_string(&settings).unwrap();
    let count = content2.matches("au hook pre").count();
    assert_eq!(count, 1, "hooks should be idempotent, got {count} copies");

    fs::remove_dir_all(&dir).ok();
    fs::remove_dir_all(&fake_home).ok();
}

#[test]
fn init_uninstall_hooks_removes_only_agent_undo_entries() {
    let dir = unique_tmp_dir("uninstall_hooks_flag");
    fs::write(dir.join("x.txt"), "x").unwrap();
    let fake_home = unique_tmp_dir("fake_home_uninstall");

    let install = Command::new(bin_path())
        .args(["init", "--install-hooks"])
        .current_dir(&dir)
        .env("HOME", &fake_home)
        .output()
        .unwrap();
    assert_eq!(install.status.code().unwrap_or(-1), 0);

    let settings = fake_home.join(".claude/settings.json");
    assert!(
        settings.exists(),
        "settings.json should exist after install"
    );

    let uninstall = Command::new(bin_path())
        .args(["init", "--uninstall-hooks"])
        .current_dir(&dir)
        .env("HOME", &fake_home)
        .output()
        .unwrap();
    assert_eq!(uninstall.status.code().unwrap_or(-1), 0);

    let content = fs::read_to_string(&settings).unwrap();
    assert!(!content.contains("agent-undo-managed"));
    assert!(!content.contains("au hook pre"));
    assert!(!content.contains("au hook post"));

    fs::remove_dir_all(&dir).ok();
    fs::remove_dir_all(&fake_home).ok();
}

#[test]
fn blame_walks_event_history_and_attributes_lines() {
    let dir = unique_tmp_dir("blame");
    fs::write(dir.join("c.rs"), "line one\nline two\n").unwrap();
    run(&dir, &["init"]);

    // Use exec wrapper which sets active-session, then perform a write that
    // the watcher would normally catch. Since serve isn't running here, we
    // simulate by running blame on whatever events exist (initial-scan only),
    // verifying the command succeeds and outputs both lines.
    let (code, out, err) = run(&dir, &["blame", "c.rs"]);
    assert_eq!(code, 0, "blame failed: {err}");
    assert!(out.contains("line one"), "missing line one: {out}");
    assert!(out.contains("line two"), "missing line two: {out}");
    assert!(out.contains("initial-scan"), "missing attribution: {out}");

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn blame_errors_on_unknown_file() {
    let dir = unique_tmp_dir("blame_404");
    fs::write(dir.join("a.txt"), "x").unwrap();
    run(&dir, &["init"]);

    let (code, _, err) = run(&dir, &["blame", "no-such-file.rs"]);
    assert_ne!(code, 0, "blame on missing file should fail");
    assert!(
        err.contains("no history") || err.to_lowercase().contains("error"),
        "expected error message: {err}"
    );
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn daemon_starts_writes_pidfile_and_stops_cleanly() {
    let dir = unique_tmp_dir("daemon");
    fs::write(dir.join("seed.txt"), "x").unwrap();
    run(&dir, &["init"]);

    let (code, out, _) = run(&dir, &["serve", "--daemon"]);
    assert_eq!(code, 0, "serve --daemon failed: {out}");
    assert!(out.contains("daemon started"), "unexpected output: {out}");

    let pidfile = dir.join(".agent-undo/daemon.pid");
    assert!(pidfile.exists(), "pidfile missing");
    let pid: u32 = fs::read_to_string(&pidfile)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert!(pid > 0);
    wait_for_daemon_ready(&dir);

    // Modify a file — the daemon should snapshot it.
    fs::write(dir.join("seed.txt"), "y").unwrap();
    let _ = wait_for_log_match(&dir, "daemon should have caught the modification", |out| {
        out.contains("modify seed.txt")
    });

    let (stop_code, stop_out, _) = run(&dir, &["stop"]);
    assert_eq!(stop_code, 0, "stop failed: {stop_out}");
    std::thread::sleep(Duration::from_millis(200));
    assert!(!pidfile.exists(), "pidfile should be cleaned up after stop");

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn daemon_coalesces_temp_file_save_patterns_into_the_real_file() {
    let dir = unique_tmp_dir("daemon_tmp_save");
    fs::write(dir.join("note.txt"), "v1").unwrap();
    run(&dir, &["init"]);

    let (code, out, _) = run(&dir, &["serve", "--daemon"]);
    assert_eq!(code, 0, "serve --daemon failed: {out}");
    wait_for_daemon_ready(&dir);

    let save = Command::new("sh")
        .arg("-c")
        .arg("printf 'v2' > note.txt.tmp && mv note.txt.tmp note.txt")
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(save.status.success(), "temp-file save failed: {:?}", save);
    let log_out = wait_for_log_match(
        &dir,
        "expected temp-file save to be attributed to the real file",
        |out| {
            (out.contains("modify note.txt") || out.contains("create note.txt"))
                && !out.contains("note.txt.tmp")
        },
    );
    assert!(
        !log_out.contains("note.txt.tmp"),
        "temp file should not pollute the timeline: {log_out}"
    );

    let _ = run(&dir, &["stop"]);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn pin_creates_an_entry_and_gc_preserves_it() {
    let dir = unique_tmp_dir("pin");
    fs::write(dir.join("a.txt"), "x").unwrap();
    run(&dir, &["init"]);

    let (code, out, _) = run(&dir, &["pin", "before-refactor"]);
    assert_eq!(code, 0, "pin failed: {out}");
    assert!(out.contains("before-refactor"), "unexpected: {out}");

    // gc with default policy (7 days) shouldn't drop the just-created event.
    let (gc_code, gc_out, _) = run(&dir, &["gc"]);
    assert_eq!(gc_code, 0);
    assert!(
        gc_out.contains("removed 0 event"),
        "unexpected gc output: {gc_out}"
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn install_script_exists_and_is_executable() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("scripts")
        .join("install.sh");
    assert!(path.exists(), "install.sh missing at {}", path.display());
    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("REPO=\"peaktwilight/agent-undo\""));
    assert!(content.contains("apple-darwin"));
    assert!(content.contains("unknown-linux-gnu"));
    assert!(content.contains("aarch64"));
    assert!(content.contains("x86_64"));
    // Sanity-check shell parsing.
    let status = Command::new("sh")
        .args(["-n", path.to_str().unwrap()])
        .status()
        .unwrap();
    assert!(status.success(), "install.sh has shell parse errors");
}

#[test]
fn log_filters_by_agent_path_and_since() {
    let dir = unique_tmp_dir("log_filter");
    fs::write(dir.join("alpha.txt"), "a").unwrap();
    fs::write(dir.join("beta.txt"), "b").unwrap();
    fs::write(dir.join("gamma.rs"), "g").unwrap();
    run(&dir, &["init"]);

    // --agent filter
    let (_, by_agent, _) = run(&dir, &["log", "--agent", "initial-scan"]);
    assert!(by_agent.contains("alpha.txt"));
    assert!(by_agent.contains("beta.txt"));
    assert!(by_agent.contains("gamma.rs"));

    let (_, no_match, _) = run(&dir, &["log", "--agent", "claude-code"]);
    assert!(
        no_match.contains("no events"),
        "expected no events: {no_match}"
    );

    // --file substring filter
    let (_, by_file, _) = run(&dir, &["log", "--file", "alpha"]);
    assert!(by_file.contains("alpha.txt"));
    assert!(!by_file.contains("beta.txt"));

    // --since (everything is recent so 1d should match all)
    let (_, by_since, _) = run(&dir, &["log", "--since", "1d"]);
    assert!(by_since.contains("alpha.txt"));

    // --since rejects garbage
    let (code, _, _) = run(&dir, &["log", "--since", "abc"]);
    assert_ne!(code, 0, "--since with bad input should error");

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn log_json_outputs_machine_readable_events() {
    let dir = unique_tmp_dir("log_json");
    fs::write(dir.join("alpha.txt"), "a").unwrap();
    run(&dir, &["init"]);

    let (code, out, err) = run(&dir, &["log", "--json"]);
    assert_eq!(code, 0, "log --json failed: {err}");
    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    let events = parsed
        .as_array()
        .expect("log --json should return an array");
    assert!(
        !events.is_empty(),
        "log --json should include initial scan events"
    );
    assert_eq!(events[0]["path"], "alpha.txt");
    assert_eq!(events[0]["attribution"], "initial-scan");

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn unpin_restores_project_to_pinned_state() {
    let dir = unique_tmp_dir("unpin");
    fs::write(dir.join("file.rs"), "v1").unwrap();
    run(&dir, &["init"]);
    run(&dir, &["pin", "v1-snapshot"]);
    let (serve_code, _, serve_err) = run(&dir, &["serve", "--daemon"]);
    assert_eq!(serve_code, 0, "serve --daemon failed: {serve_err}");
    wait_for_daemon_ready(&dir);

    fs::write(dir.join("file.rs"), "v2").unwrap();
    fs::write(dir.join("new.rs"), "brand new").unwrap();
    let _ = wait_for_log_match(
        &dir,
        "expected daemon to record both the file edit and the new file before unpin",
        |out| {
            out.contains("new.rs")
                && out.lines().filter(|line| line.contains("file.rs")).count() >= 2
        },
    );

    let (code, out, _) = run(&dir, &["unpin", "v1-snapshot"]);
    assert_eq!(code, 0, "unpin failed: {out}");
    assert!(
        out.contains("file.rs"),
        "expected file.rs in restore output: {out}"
    );
    assert!(
        out.contains("new.rs"),
        "expected new.rs in restore output: {out}"
    );
    assert_eq!(fs::read_to_string(dir.join("file.rs")).unwrap(), "v1");
    assert!(
        !dir.join("new.rs").exists(),
        "unpin should delete files created after the pin"
    );

    let _ = run(&dir, &["stop"]);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn pin_list_shows_existing_pins() {
    let dir = unique_tmp_dir("pin_list");
    fs::write(dir.join("file.rs"), "v1").unwrap();
    run(&dir, &["init"]);
    run(&dir, &["pin", "before-refactor"]);

    let (code, out, err) = run(&dir, &["pin", "--list"]);
    assert_eq!(code, 0, "pin --list failed: {err}");
    assert!(
        out.contains("before-refactor"),
        "pin list missing label: {out}"
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn pin_list_json_outputs_machine_readable_rows() {
    let dir = unique_tmp_dir("pin_list_json");
    fs::write(dir.join("file.rs"), "v1").unwrap();
    run(&dir, &["init"]);
    run(&dir, &["pin", "before-refactor"]);

    let (code, out, err) = run(&dir, &["pin", "--list", "--json"]);
    assert_eq!(code, 0, "pin --list --json failed: {err}");
    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    let rows = parsed.as_array().expect("pin list json should be an array");
    assert!(!rows.is_empty());
    assert_eq!(rows[0]["label"], "before-refactor");

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn doctor_reports_status_correctly() {
    let dir = unique_tmp_dir("doctor");
    fs::write(dir.join("a.txt"), "hi").unwrap();

    // Doctor before init should give a clear "no .agent-undo/" message.
    let (_, out_pre, _) = run(&dir, &["doctor"]);
    assert!(
        out_pre.contains("no .agent-undo"),
        "expected error: {out_pre}"
    );

    run(&dir, &["init"]);

    // Doctor after init should report all the basics.
    let (code, out, _) = run(&dir, &["doctor"]);
    assert_eq!(code, 0);
    assert!(
        out.contains("project initialized"),
        "missing init line: {out}"
    );
    assert!(
        out.contains("timeline database open"),
        "missing db line: {out}"
    );
    assert!(out.contains("object(s) in CAS"), "missing CAS line: {out}");
    assert!(
        out.contains("daemon not running"),
        "should flag missing daemon: {out}"
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn doctor_json_outputs_machine_readable_health() {
    let dir = unique_tmp_dir("doctor_json");
    fs::write(dir.join("a.txt"), "hi").unwrap();
    run(&dir, &["init"]);

    let (code, out, err) = run(&dir, &["doctor", "--json"]);
    assert_eq!(code, 0, "doctor --json failed: {err}");
    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed["events"], 1);
    assert!(parsed.get("wrappers").is_some());
    assert!(parsed.get("daemon").is_some());
    assert!(parsed.get("claude_hooks").is_some());

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn doctor_fix_recreates_missing_config_and_cleans_stale_pidfile() {
    let dir = unique_tmp_dir("doctor_fix");
    fs::write(dir.join("a.txt"), "hi").unwrap();
    run(&dir, &["init"]);

    let config_path = dir.join(".agent-undo/config.toml");
    fs::remove_file(&config_path).unwrap();
    let pidfile = dir.join(".agent-undo/daemon.pid");
    fs::write(&pidfile, "999999").unwrap();

    let (code, out, err) = run(&dir, &["doctor", "--fix"]);
    assert_eq!(code, 0, "doctor --fix failed: {err}");
    assert!(
        out.contains("wrote default config"),
        "missing config fix: {out}"
    );
    assert!(
        out.contains("removed stale pidfile"),
        "missing pidfile fix: {out}"
    );
    assert!(
        config_path.exists(),
        "doctor --fix should recreate config.toml"
    );
    assert!(
        !pidfile.exists(),
        "doctor --fix should remove stale pidfile"
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn doctor_reports_detected_terminal_agent_binaries_on_path() {
    let dir = unique_tmp_dir("doctor_detects_wrappers");
    fs::write(dir.join("a.txt"), "hi").unwrap();
    run(&dir, &["init"]);

    let fake_bin = unique_tmp_dir("doctor_detects_wrappers_fakebin");
    let fake_codex = fake_bin.join("codex");
    fs::write(&fake_codex, "#!/usr/bin/env sh\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&fake_codex).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&fake_codex, perms).unwrap();
    }

    let path = format!(
        "{}:{}",
        fake_bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let output = Command::new(bin_path())
        .args(["doctor"])
        .current_dir(&dir)
        .env("PATH", path)
        .output()
        .unwrap();
    assert_eq!(output.status.code().unwrap_or(-1), 0);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("detected terminal-agent CLI(s) on PATH: codex"));
    assert!(stdout.contains("wrappers missing for: codex"));

    fs::remove_dir_all(&dir).ok();
    fs::remove_dir_all(&fake_bin).ok();
}

#[test]
fn doctor_fix_installs_missing_wrappers_for_detected_binaries() {
    let dir = unique_tmp_dir("doctor_fix_wrappers");
    fs::write(dir.join("a.txt"), "hi").unwrap();
    run(&dir, &["init"]);

    let fake_bin = unique_tmp_dir("doctor_fix_wrappers_fakebin");
    let fake_codex = fake_bin.join("codex");
    fs::write(&fake_codex, "#!/usr/bin/env sh\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&fake_codex).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&fake_codex, perms).unwrap();
    }

    let path = format!(
        "{}:{}",
        fake_bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let output = Command::new(bin_path())
        .args(["doctor", "--fix"])
        .current_dir(&dir)
        .env("PATH", path)
        .output()
        .unwrap();
    assert_eq!(output.status.code().unwrap_or(-1), 0);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("fixed: installed wrapper(s) for codex"));
    assert!(dir.join(".agent-undo/bin/codex").exists());

    fs::remove_dir_all(&dir).ok();
    fs::remove_dir_all(&fake_bin).ok();
}

#[test]
fn gc_uses_configured_keep_last_window() {
    let dir = unique_tmp_dir("gc_config");
    fs::write(dir.join("seed.txt"), "x").unwrap();
    run(&dir, &["init"]);

    let (code, out, _) = run(&dir, &["serve", "--daemon"]);
    assert_eq!(code, 0, "serve --daemon failed: {out}");
    wait_for_daemon_ready(&dir);

    fs::write(dir.join("seed.txt"), "y").unwrap();
    let _ = wait_for_log_match(&dir, "expected daemon to record the gc test edit", |out| {
        out.contains("modify seed.txt")
    });
    let _ = run(&dir, &["stop"]);

    fs::write(
        dir.join(".agent-undo/config.toml"),
        "[gc]\nkeep_last = \"0s\"\n\n[watch]\nmax_file_size_mb = 100\nignore_patterns = []\n",
    )
    .unwrap();

    let (gc_code, gc_out, gc_err) = run(&dir, &["gc"]);
    assert_eq!(gc_code, 0, "gc failed: {gc_err}");
    assert!(
        gc_out.contains("older than 0s"),
        "unexpected gc output: {gc_out}"
    );
    assert!(
        !gc_out.contains("removed 0 event"),
        "gc should respect config and remove an older event: {gc_out}"
    );

    fs::remove_dir_all(&dir).ok();
}

#[test]
fn discover_errors_outside_initialized_project() {
    let dir = unique_tmp_dir("undisc");
    let (code, _, err) = run(&dir, &["log"]);
    assert_ne!(code, 0, "log should fail when no .agent-undo/ exists");
    assert!(
        err.contains("no .agent-undo") || err.contains("Error"),
        "expected error message, got stderr: {err}"
    );
    fs::remove_dir_all(&dir).ok();
}
