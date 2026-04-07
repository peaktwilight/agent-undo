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
mod paths;
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
    Init,

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
        Command::Init => cmd_init(),
        Command::Status => cmd_status(),
        Command::Serve => cmd_serve(),
        Command::Log { limit, .. } => cmd_log(limit),
        Command::Sessions => not_impl("sessions"),
        Command::Diff { .. } => not_impl("diff"),
        Command::Show { .. } => not_impl("show"),
        Command::Restore { .. } => not_impl("restore"),
        Command::Oops { .. } => not_impl("oops"),
        Command::Pin { .. } => not_impl("pin"),
        Command::Blame { .. } => not_impl("blame (v2)"),
        Command::Tui => not_impl("tui"),
        Command::Exec { .. } => not_impl("exec"),
        Command::Session(_) => not_impl("session"),
        Command::Hook(_) => not_impl("hook"),
        Command::Gc => not_impl("gc"),
    }
}

fn not_impl(name: &str) -> Result<()> {
    println!("agent-undo {name}: not yet implemented");
    Ok(())
}

// --- commands --------------------------------------------------------------

fn cmd_init() -> Result<()> {
    let paths = ProjectPaths::cwd_as_root()?;
    if paths.data_dir.exists() {
        println!(
            "agent-undo is already initialized at {}",
            paths.data_dir.display()
        );
        return Ok(());
    }

    println!("Initializing agent-undo at {}", paths.root.display());
    let store = Store::init(paths)?;

    println!("Scanning project files...");
    let count = daemon::initial_scan(&store)?;
    println!("  snapshotted {count} files");

    println!();
    println!("Next:");
    println!("  agent-undo serve    # start the watcher");
    println!("  agent-undo log      # see events as they happen");
    println!("  agent-undo oops     # panic button (coming soon)");
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
