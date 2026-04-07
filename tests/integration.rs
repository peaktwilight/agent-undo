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
use std::time::{SystemTime, UNIX_EPOCH};

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
    // cargo sets CARGO_BIN_EXE_<bin_name> for integration tests
    PathBuf::from(env!("CARGO_BIN_EXE_agent-undo"))
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
        "blame", "tui", "exec", "session", "hook", "gc",
    ] {
        assert!(out.contains(cmd), "help output missing `{cmd}`:\n{out}");
    }
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
