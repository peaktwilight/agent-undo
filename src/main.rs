// agent-undo — Ctrl-Z for AI coding agents.
//
// CLI entry point. Modules:
//
//   paths  — project path discovery (.agent-undo/ layout)
//   store  — content-addressable blob store + SQLite timeline
//   daemon — FS watcher pipeline and initial scan
//
// Milestone A (this file): `init`, `serve`, `log` end-to-end.
// Later milestones add `restore`, `oops`, attribution, sessions, TUI, hooks.

mod daemon;
mod hook;
mod install;
mod paths;
mod restore;
mod store;

use anyhow::Result;
use chrono::{Local, TimeZone};
use clap::{Parser, Subcommand};

use crate::paths::ProjectPaths;
use crate::store::Store;

#[derive(Parser)]
#[command(
    name = "agent-undo",
    version,
    about = "Ctrl-Z for AI coding agents",
    long_about = "Snapshots every file your AI coding agent touches, attributes edits \
                  to specific agents (Claude Code, Cursor, Cline, Aider, Codex), and \
                  lets you undo any session with one command."
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

    /// Start the watcher (foreground). Usually run once via `init` in v0.
    Serve,

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
        Command::Serve => cmd_serve(),
        Command::Log { limit, .. } => cmd_log(limit),
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
        Command::Pin { .. } => not_impl("pin"),
        Command::Blame { .. } => not_impl("blame (v2)"),
        Command::Tui => not_impl("tui"),
        Command::Exec {
            agent,
            label,
            command,
        } => cmd_exec(agent, label, command),
        Command::Session(_) => not_impl("session"),
        Command::Hook(HookCmd::Pre) => hook::handle_pre(),
        Command::Hook(HookCmd::Post) => hook::handle_post(),
        Command::Gc => not_impl("gc"),
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
        println!("  agent-undo serve    # start the watcher");
        println!("  agent-undo log      # see events as they happen");
        println!("  agent-undo oops     # panic button");
        if !install_hooks {
            println!();
            println!(
                "Tip: run `agent-undo init --install-hooks` to auto-attribute Claude Code edits."
            );
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

fn cmd_serve() -> Result<()> {
    let paths = ProjectPaths::discover()?;
    let store = Store::open(paths)?;
    daemon::serve(store)
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

fn cmd_log(limit: usize) -> Result<()> {
    let paths = ProjectPaths::discover()?;
    let store = Store::open(paths)?;
    let events = store.recent_events(limit)?;

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
