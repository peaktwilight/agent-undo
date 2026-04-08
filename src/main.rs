// agent-undo (binary: `au`) — Ctrl-Z for AI coding agents.
//
// The crate ships as `agent-undo` on crates.io for discoverability and
// panic-search ("how do I undo a Claude Code edit"), but installs a short
// `au` binary in PATH — same shape as `ripgrep` installing `rg`.
//
// CLI entry point. Modules:
//
//   paths  — project path discovery (.agent-undo/ layout)
//   store  — content-addressable blob store + SQLite timeline
//   daemon — FS watcher pipeline and initial scan
//
// All user-facing commands run as `au <subcommand>`. The storage dir stays
// `.agent-undo/` so anyone walking past a colleague's screen knows what it is.

mod blame;
mod daemon;
mod hook;
mod install;
mod paths;
mod restore;
mod store;
mod tui;

use anyhow::Result;
use chrono::{Local, TimeZone};
use clap::{Parser, Subcommand};

use crate::paths::ProjectPaths;
use crate::store::Store;

#[derive(Parser)]
#[command(
    name = "au",
    version,
    about = "agent-undo — Ctrl-Z for AI coding agents",
    long_about = "agent-undo (au) snapshots every file your AI coding agent touches, \
                  attributes edits to specific agents (Claude Code, Cursor, Cline, Aider, \
                  Codex), and lets you undo any session with one command. Local-first, \
                  zero telemetry, single 3.9 MB binary."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize agent-undo in the current project.
    Init {
        /// Also patch ~/.claude/settings.json with agent-undo hooks so
        /// Claude Code edits are attributed automatically.
        #[arg(long)]
        install_hooks: bool,

        /// Skip initial project scan (fast init).
        #[arg(long)]
        no_scan: bool,
    },

    /// Show daemon status and recent activity.
    Status,

    /// Start the watcher.
    Serve {
        /// Detach into the background and write a pidfile to .agent-undo/daemon.pid.
        #[arg(long)]
        daemon: bool,
    },

    /// Stop a running background daemon (reads .agent-undo/daemon.pid).
    Stop,

    /// Show the timeline of file events.
    Log {
        #[arg(long)]
        agent: Option<String>,
        #[arg(long)]
        file: Option<String>,
        #[arg(long)]
        since: Option<String>,
        #[arg(short = 'n', long, default_value_t = 50)]
        limit: usize,
    },

    /// List agent sessions.
    Sessions,

    /// Show the diff for a single event or an entire session.
    Diff {
        event_id: Option<u64>,
        #[arg(long)]
        session: Option<String>,
    },

    /// Print file contents at a point in time.
    Show {
        event_id: u64,
        #[arg(long)]
        before: bool,
        #[arg(long)]
        after: bool,
    },

    /// Restore a file to a previous state.
    Restore {
        event_id: Option<u64>,
        #[arg(long)]
        file: Option<String>,
        #[arg(long)]
        session: Option<String>,
    },

    /// Panic button — undo the last agent action.
    Oops {
        #[arg(long)]
        confirm: bool,
    },

    /// Pin the current state so it's never garbage collected.
    Pin { label: String },

    /// Restore the project to a previously pinned state.
    Unpin { label: String },

    /// Show per-line agent attribution for a file (v2).
    Blame { file: String },

    /// Interactive timeline browser.
    Tui,

    /// Run a command and attribute all its file writes to a specific agent.
    Exec {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        label: Option<String>,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },

    /// Session lifecycle (used by shims).
    #[command(subcommand)]
    Session(SessionCmd),

    /// Claude Code hook handler — reads JSON from stdin.
    #[command(subcommand)]
    Hook(HookCmd),

    /// Garbage collect old events and blobs.
    Gc,

    /// Diagnose the project's agent-undo setup.
    Doctor,
}

#[derive(Subcommand)]
enum SessionCmd {
    Start {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        metadata: Option<String>,
    },
    End {
        session_id: String,
    },
}

#[derive(Subcommand)]
enum HookCmd {
    Pre,
    Post,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "agent_undo=info".into()),
        )
        .with_target(false)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Init {
            install_hooks,
            no_scan,
        } => cmd_init(install_hooks, no_scan),
        Command::Status => cmd_status(),
        Command::Serve { daemon } => cmd_serve(daemon),
        Command::Stop => cmd_stop(),
        Command::Log {
            agent,
            file,
            since,
            limit,
        } => cmd_log(agent, file, since, limit),
        Command::Sessions => cmd_sessions(),
        Command::Diff { event_id, session } => cmd_diff(event_id, session),
        Command::Show {
            event_id,
            before,
            after,
        } => cmd_show(event_id, before, after),
        Command::Restore {
            event_id,
            file,
            session,
        } => cmd_restore(event_id, file, session),
        Command::Oops { confirm } => cmd_oops(confirm),
        Command::Pin { label } => cmd_pin(label),
        Command::Unpin { label } => cmd_unpin(label),
        Command::Blame { file } => cmd_blame(file),
        Command::Tui => cmd_tui(),
        Command::Exec {
            agent,
            label,
            command,
        } => cmd_exec(agent, label, command),
        Command::Session(_) => not_impl("session"),
        Command::Hook(HookCmd::Pre) => hook::handle_pre(),
        Command::Hook(HookCmd::Post) => hook::handle_post(),
        Command::Gc => cmd_gc(),
        Command::Doctor => cmd_doctor(),
    }
}

fn not_impl(name: &str) -> Result<()> {
    println!("agent-undo {name}: not yet implemented");
    Ok(())
}

// --- commands --------------------------------------------------------------

fn cmd_init(install_hooks: bool, no_scan: bool) -> Result<()> {
    let paths = ProjectPaths::cwd_as_root()?;
    let fresh = !paths.data_dir.exists();

    if fresh {
        println!("Initializing agent-undo at {}", paths.root.display());
        let store = Store::init(paths.clone())?;

        if !no_scan {
            println!("Scanning project files...");
            let count = daemon::initial_scan(&store)?;
            println!("  snapshotted {count} files");
        } else {
            println!("  (skipping initial scan — --no-scan)");
        }
    } else {
        println!(
            "agent-undo is already initialized at {}",
            paths.data_dir.display()
        );
    }

    if install_hooks {
        match install::install_claude_hooks() {
            Ok((true, path)) => {
                println!();
                println!("✓ installed Claude Code hooks into {}", path.display());
                println!("  Claude Code edits will now be attributed automatically.");
                println!("  Restart any open Claude Code sessions to pick them up.");
            }
            Ok((false, path)) => {
                println!();
                println!("✓ Claude Code hooks already present in {}", path.display());
            }
            Err(e) => {
                eprintln!();
                eprintln!("⚠ could not install Claude Code hooks: {e}");
                eprintln!("  init still succeeded — attribution will fall back to 'unknown'.");
            }
        }
    }

    if fresh {
        println!();
        println!("Next:");
        println!("  au serve --daemon   # start the watcher in the background");
        println!("  au log              # see events as they happen");
        println!("  au oops             # panic button");
        if !install_hooks {
            println!();
            println!("Tip: run `au init --install-hooks` to auto-attribute Claude Code edits.");
        }
    }
    Ok(())
}

fn cmd_status() -> Result<()> {
    let paths = match ProjectPaths::discover() {
        Ok(p) => p,
        Err(e) => {
            println!("{e}");
            return Ok(());
        }
    };
    let store = Store::open(paths.clone())?;
    let events = store.event_count()?;
    println!("root:     {}", paths.root.display());
    println!("data:     {}", paths.data_dir.display());
    println!("events:   {events}");
    println!("database: {}", paths.db_path.display());
    Ok(())
}

fn cmd_serve(detach: bool) -> Result<()> {
    let paths = ProjectPaths::discover()?;

    if detach {
        return spawn_daemon(&paths);
    }

    // Foreground mode: write pidfile so `stop` can find us, clean up on exit.
    let pidfile = paths.data_dir.join("daemon.pid");
    let _ = std::fs::write(&pidfile, std::process::id().to_string());
    let store = Store::open(paths)?;
    let result = daemon::serve(store);
    let _ = std::fs::remove_file(&pidfile);
    result
}

fn spawn_daemon(paths: &ProjectPaths) -> Result<()> {
    use std::process::{Command as Proc, Stdio};

    let pidfile = paths.data_dir.join("daemon.pid");
    if let Some(existing_pid) = read_pidfile(&pidfile) {
        if process_alive(existing_pid) {
            println!(
                "agent-undo daemon already running (pid {existing_pid}). Use `agent-undo stop` to stop it."
            );
            return Ok(());
        } else {
            // stale pidfile
            let _ = std::fs::remove_file(&pidfile);
        }
    }

    let log_path = paths.data_dir.join("daemon.log");
    let log = std::fs::File::create(&log_path)?;
    let log_err = log.try_clone()?;

    let exe = std::env::current_exe()?;
    let child = Proc::new(exe)
        .args(["serve"])
        .current_dir(&paths.root)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(log_err))
        .spawn()?;

    // Write the pidfile from the parent so it exists before we return.
    // The child also writes it on its own startup; that's a harmless rewrite.
    std::fs::write(&pidfile, child.id().to_string())?;

    println!(
        "agent-undo daemon started (pid {}). Logs: {}",
        child.id(),
        log_path.display()
    );
    println!("Stop with: agent-undo stop");
    Ok(())
}

fn cmd_stop() -> Result<()> {
    let paths = ProjectPaths::discover()?;
    let pidfile = paths.data_dir.join("daemon.pid");
    let pid = match read_pidfile(&pidfile) {
        Some(p) => p,
        None => {
            println!("no daemon running for {}", paths.root.display());
            return Ok(());
        }
    };
    if !process_alive(pid) {
        println!("stale pidfile for pid {pid}; cleaning up");
        let _ = std::fs::remove_file(&pidfile);
        return Ok(());
    }
    #[cfg(unix)]
    {
        let _ = std::process::Command::new("kill")
            .arg(pid.to_string())
            .status();
        println!("sent SIGTERM to agent-undo daemon (pid {pid})");
    }
    #[cfg(not(unix))]
    {
        println!("stop on non-unix platforms not yet supported (kill pid {pid} manually)");
    }
    let _ = std::fs::remove_file(&pidfile);
    Ok(())
}

fn read_pidfile(path: &std::path::Path) -> Option<u32> {
    let s = std::fs::read_to_string(path).ok()?;
    s.trim().parse::<u32>().ok()
}

fn process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        // signal 0 doesn't kill, just probes whether the process exists
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}

/// Parse a relative duration string like "5m", "2h", "1d" into nanoseconds.
fn parse_since(s: &str) -> Result<i64> {
    let s = s.trim();
    if s.is_empty() {
        anyhow::bail!("--since cannot be empty (try '5m', '2h', '1d')");
    }
    let (num_str, unit) = s.split_at(s.len() - 1);
    let n: i64 = num_str
        .parse()
        .map_err(|_| anyhow::anyhow!("could not parse --since '{s}': try '5m', '2h', '1d'"))?;
    let mult = match unit {
        "s" => 1_000_000_000,
        "m" => 60 * 1_000_000_000,
        "h" => 60 * 60 * 1_000_000_000,
        "d" => 24 * 60 * 60 * 1_000_000_000_i64,
        _ => anyhow::bail!("unknown --since unit '{unit}', use s/m/h/d"),
    };
    Ok(n * mult)
}

fn cmd_doctor() -> Result<()> {
    println!("agent-undo doctor");
    println!("=================");
    println!();

    // 1. Project init.
    let paths = match ProjectPaths::discover() {
        Ok(p) => {
            println!("✓ project initialized at {}", p.root.display());
            p
        }
        Err(_) => {
            println!("✗ no .agent-undo/ found in this directory or any parent");
            println!("  → run `agent-undo init` to set up");
            return Ok(());
        }
    };

    // 2. SQLite store + counts.
    let store = match Store::open(paths.clone()) {
        Ok(s) => {
            let n = s.event_count().unwrap_or(-1);
            println!("✓ timeline database open ({n} events)");
            s
        }
        Err(e) => {
            println!("✗ could not open timeline.db: {e}");
            return Ok(());
        }
    };

    // 3. Object store on disk.
    let blob_count = count_blobs(&paths.objects_dir);
    println!(
        "✓ {blob_count} object(s) in CAS at {}",
        paths.objects_dir.display()
    );

    // 4. Daemon status.
    let pidfile = paths.data_dir.join("daemon.pid");
    if pidfile.exists() {
        let pid_str = std::fs::read_to_string(&pidfile).unwrap_or_default();
        let pid: u32 = pid_str.trim().parse().unwrap_or(0);
        if pid > 0 && process_alive(pid) {
            println!("✓ daemon running (pid {pid})");
        } else {
            println!(
                "⚠ stale pidfile at {} (pid {pid} not alive)",
                pidfile.display()
            );
            println!("  → run `au stop` to clean up, then `au serve --daemon`");
        }
    } else {
        println!("⚠ daemon not running");
        println!("  → start it with `au serve --daemon`");
    }

    // 5. Active session marker.
    let active_path = paths.data_dir.join("active-session.json");
    if active_path.exists() {
        println!(
            "ℹ active session marker present at {}",
            active_path.display()
        );
        println!("  (this means an agent is currently making attributed edits)");
    }

    // 6. Claude Code hooks installed?
    if let Some(settings) = install::claude_settings_path() {
        if settings.exists() {
            let content = std::fs::read_to_string(&settings).unwrap_or_default();
            if content.contains("au hook") || content.contains("agent-undo hook") {
                println!("✓ Claude Code hooks installed in {}", settings.display());
            } else {
                println!("⚠ Claude Code settings.json exists but no agent-undo hook found");
                println!("  → run `au init --install-hooks` to add them");
            }
        } else {
            println!(
                "ℹ Claude Code not detected (no {} found)",
                settings.display()
            );
            println!("  → if you use Claude Code, run `au init --install-hooks`");
        }
    }

    // 7. Sessions summary.
    let sessions = store.list_sessions(5).unwrap_or_default();
    println!();
    if sessions.is_empty() {
        println!("no agent sessions recorded yet.");
    } else {
        println!("recent sessions:");
        for s in sessions {
            let when = chrono::Local
                .timestamp_nanos(s.started_at_ns)
                .format("%Y-%m-%d %H:%M")
                .to_string();
            println!("  {} {} — {}", &s.id[..s.id.len().min(12)], s.agent, when);
        }
    }

    println!();
    println!("everything looks healthy. try `au log` to see what's happening.");
    Ok(())
}

fn count_blobs(objects_dir: &std::path::Path) -> usize {
    let mut count = 0;
    if let Ok(shards) = std::fs::read_dir(objects_dir) {
        for shard in shards.flatten() {
            if shard.path().is_dir() {
                if let Ok(blobs) = std::fs::read_dir(shard.path()) {
                    count += blobs.flatten().count();
                }
            }
        }
    }
    count
}

fn cmd_pin(label: String) -> Result<()> {
    let paths = ProjectPaths::discover()?;
    let store = Store::open(paths)?;
    let id = store.create_pin(&label)?;
    println!("pinned current state as #{id}: {label}");
    Ok(())
}

fn cmd_unpin(label: String) -> Result<()> {
    let paths = ProjectPaths::discover()?;
    let store = Store::open(paths)?;
    let restored = restore::restore_pin(&store, &label)?;
    if restored.is_empty() {
        println!("pin '{label}' has no recorded state to restore");
    } else {
        println!(
            "restored project to pin '{}' — {} file(s):",
            label,
            restored.len()
        );
        for p in restored.iter().take(20) {
            println!("  {p}");
        }
        if restored.len() > 20 {
            println!("  ... and {} more", restored.len() - 20);
        }
    }
    Ok(())
}

fn cmd_gc() -> Result<()> {
    // Default policy: keep last 7 days of events.
    const SEVEN_DAYS_NS: i64 = 7 * 24 * 60 * 60 * 1_000_000_000;
    let paths = ProjectPaths::discover()?;
    let store = Store::open(paths)?;
    let (events, blobs) = store.gc(SEVEN_DAYS_NS)?;
    println!("gc: removed {events} event(s) and {blobs} blob(s) older than 7 days");
    println!("    (pinned events and the latest event per file are always preserved)");
    Ok(())
}

fn cmd_blame(file: String) -> Result<()> {
    let paths = ProjectPaths::discover()?;
    let store = Store::open(paths)?;
    blame::blame(&store, &file)
}

fn cmd_tui() -> Result<()> {
    let paths = ProjectPaths::discover()?;
    let store = Store::open(paths)?;
    tui::run(&store)
}

fn cmd_exec(agent: String, label: Option<String>, command: Vec<String>) -> Result<()> {
    use std::process::Command as Proc;
    use std::time::{SystemTime, UNIX_EPOCH};

    if command.is_empty() {
        anyhow::bail!("usage: agent-undo exec --agent <name> -- <command...>");
    }

    let paths = ProjectPaths::discover()?;
    let store = Store::open(paths.clone())?;
    let ts_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0);
    let session_id = format!("exec-{}", uuid::Uuid::new_v4().simple());

    // Start the session record.
    store.upsert_session(&store::SessionRow {
        id: session_id.clone(),
        agent: agent.clone(),
        started_at_ns: ts_ns,
        ended_at_ns: None,
        prompt: label.clone(),
        model: None,
        metadata: Some(serde_json::json!({ "command": command }).to_string()),
    })?;

    // Write active-session marker so the watcher attributes child writes.
    hook::write_active_session(
        &paths,
        Some(&hook::ActiveSession {
            session_id: session_id.clone(),
            agent: agent.clone(),
            started_at_ns: ts_ns,
            tool_name: None,
            intended_file: None,
        }),
    )?;

    println!(
        "agent-undo exec: running as session {} (agent={})",
        &session_id[..16],
        agent
    );

    // Run the command, blocking until it exits. The watcher (assumed running
    // separately) will attribute any file writes during this window.
    let (cmd, args) = command.split_first().unwrap();
    let status = Proc::new(cmd).args(args).status();

    // Always clean up, even on error.
    let _ = hook::write_active_session(&paths, None);
    let end_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0);
    let _ = store.mark_session_ended(&session_id, end_ns);

    match status {
        Ok(s) if s.success() => {
            println!("agent-undo exec: session closed ({})", &session_id[..16]);
            Ok(())
        }
        Ok(s) => {
            let code = s.code().unwrap_or(-1);
            std::process::exit(code);
        }
        Err(e) => Err(e.into()),
    }
}

fn cmd_sessions() -> Result<()> {
    let paths = ProjectPaths::discover()?;
    let store = Store::open(paths)?;
    let sessions = store.list_sessions(50)?;
    if sessions.is_empty() {
        println!("no sessions recorded yet.");
        return Ok(());
    }
    for s in sessions {
        let start = Local
            .timestamp_nanos(s.started_at_ns)
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();
        let end = s
            .ended_at_ns
            .map(|t| Local.timestamp_nanos(t).format("%H:%M:%S").to_string())
            .unwrap_or_else(|| "(open)".into());
        println!(
            "{id}  {agent:<12}  {start} → {end}",
            id = &s.id[..s.id.len().min(12)],
            agent = s.agent,
            start = start,
            end = end,
        );
    }
    Ok(())
}

fn cmd_show(event_id: u64, before: bool, after: bool) -> Result<()> {
    let paths = ProjectPaths::discover()?;
    let store = Store::open(paths)?;
    restore::show_event(&store, event_id as i64, before, after)
}

fn cmd_diff(event_id: Option<u64>, session: Option<String>) -> Result<()> {
    let paths = ProjectPaths::discover()?;
    let store = Store::open(paths)?;
    match (event_id, session) {
        (Some(id), _) => restore::diff_event(&store, id as i64),
        (None, Some(session_id)) => {
            let events = store.events_for_session(&session_id)?;
            if events.is_empty() {
                println!("no events found for session {session_id}");
                return Ok(());
            }
            println!("# session {} — {} event(s)", session_id, events.len());
            for ev in events {
                println!();
                restore::diff_event(&store, ev.id)?;
            }
            Ok(())
        }
        (None, None) => {
            println!("usage: agent-undo diff <event-id>  OR  agent-undo diff --session <id>");
            Ok(())
        }
    }
}

fn cmd_restore(event_id: Option<u64>, file: Option<String>, session: Option<String>) -> Result<()> {
    let paths = ProjectPaths::discover()?;
    let store = Store::open(paths)?;

    match (event_id, file, session) {
        (Some(id), _, _) => {
            let ev = store
                .get_event(id as i64)?
                .ok_or_else(|| anyhow::anyhow!("no event #{id}"))?;
            restore::restore_to_event(&store, &ev)?;
            println!(
                "restored {} to state before event #{} ({})",
                ev.path,
                ev.id,
                ev.before_hash
                    .as_deref()
                    .map(|h| &h[..12])
                    .unwrap_or("<did not exist>")
            );
            Ok(())
        }
        (None, Some(path), _) => {
            let ev = restore::restore_latest_change_to_file(&store, &path)?;
            println!(
                "restored {} — undid event #{} (attribution: {})",
                path, ev.id, ev.attribution
            );
            Ok(())
        }
        (None, None, Some(session_id)) => {
            let restored = restore::restore_session(&store, &session_id)?;
            if restored.is_empty() {
                println!("no events found for session {session_id}");
            } else {
                println!(
                    "rolled back session {} — restored {} file(s):",
                    session_id,
                    restored.len()
                );
                for p in &restored {
                    println!("  {p}");
                }
            }
            Ok(())
        }
        (None, None, None) => {
            println!(
                "usage: agent-undo restore <event-id>\n   or: agent-undo restore --file <path>\n   or: agent-undo restore --session <id>"
            );
            Ok(())
        }
    }
}

fn cmd_oops(confirm: bool) -> Result<()> {
    use dialoguer::Confirm;
    const WINDOW_NS: i64 = 30 * 1_000_000_000; // 30 seconds

    let paths = ProjectPaths::discover()?;
    let store = Store::open(paths)?;

    let plan = restore::oops_plan(&store, WINDOW_NS)?;
    if plan.is_empty() {
        println!("nothing to undo — no recent user events.");
        return Ok(());
    }

    println!("agent-undo would undo the following:");
    println!();
    for (path, ev) in &plan {
        let kind = match (&ev.before_hash, &ev.after_hash) {
            (None, Some(_)) => "created",
            (Some(_), Some(_)) => "modified",
            (Some(_), None) => "deleted",
            (None, None) => "?",
        };
        println!(
            "  {:10}  {}  (event #{}, agent: {})",
            kind, path, ev.id, ev.attribution
        );
    }
    println!();

    let proceed = if confirm {
        true
    } else {
        Confirm::new()
            .with_prompt(format!("Roll back these {} file(s)?", plan.len()))
            .default(true)
            .interact()
            .unwrap_or(false)
    };

    if !proceed {
        println!("aborted.");
        return Ok(());
    }

    let done = restore::oops(&store, WINDOW_NS)?;
    println!();
    println!("restored {} file(s):", done.len());
    for (path, _) in &done {
        println!("  {path}");
    }
    println!();
    println!("tip: run `agent-undo log` to see the restore events — undo-the-undo is always one command away.");
    Ok(())
}

fn cmd_log(
    agent: Option<String>,
    file: Option<String>,
    since: Option<String>,
    limit: usize,
) -> Result<()> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let paths = ProjectPaths::discover()?;
    let store = Store::open(paths)?;

    let since_ns = match since.as_deref() {
        Some(s) => Some(parse_since(s)?),
        None => None,
    };
    // Convert relative duration (nanos before now) to absolute ts_ns.
    let abs_since = since_ns.map(|ns| {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as i64)
            .unwrap_or(0);
        now - ns
    });

    let events = store.filtered_events(agent.as_deref(), file.as_deref(), abs_since, limit)?;

    if events.is_empty() {
        println!("no events yet. run `agent-undo serve` and edit a file.");
        return Ok(());
    }

    for e in events.iter().rev() {
        let when = Local
            .timestamp_nanos(e.ts_ns)
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();
        let kind = match (&e.before_hash, &e.after_hash) {
            (None, Some(_)) => "create",
            (Some(_), Some(_)) => "modify",
            (Some(_), None) => "delete",
            (None, None) => "?",
        };
        let short_hash = e
            .after_hash
            .as_deref()
            .or(e.before_hash.as_deref())
            .map(|h| &h[..h.len().min(12)])
            .unwrap_or("-");
        println!(
            "#{id:<5} {when}  {kind:<6} {path}  [{agent}]  {hash}",
            id = e.id,
            when = when,
            kind = kind,
            path = e.path,
            agent = e.attribution,
            hash = short_hash,
        );
    }
    Ok(())
}
