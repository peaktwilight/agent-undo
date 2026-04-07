// agent-undo — Ctrl-Z for AI coding agents.
//
// This is the CLI entry point. The actual work lives in modules:
//
//   daemon/     — long-running background process (FS watcher, hasher, store, attribution)
//   store/      — content-addressable object store + SQLite timeline
//   cli/        — subcommand implementations
//   hook/       — Claude Code hook handler (stdin JSON protocol)
//   session/    — session lifecycle + attribution
//   ipc/        — unix socket protocol between CLI and daemon
//
// The "oops" command is the reason this project exists. Everything else is
// infrastructure that makes "oops" reliable.

use clap::{Parser, Subcommand};

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
    /// Initialize agent-undo in the current project and start the daemon.
    Init,

    /// Show daemon status and recent activity.
    Status,

    /// Start the daemon (usually auto-started by `init`).
    Serve,

    /// Show the timeline of file events.
    Log {
        /// Filter by agent (claude-code, cursor, cline, aider, codex, human).
        #[arg(long)]
        agent: Option<String>,

        /// Filter by file path.
        #[arg(long)]
        file: Option<String>,

        /// Show events since a time (e.g. "5m", "2h", "1d").
        #[arg(long)]
        since: Option<String>,

        /// Maximum events to show.
        #[arg(short = 'n', long, default_value_t = 50)]
        limit: usize,
    },

    /// List agent sessions.
    Sessions,

    /// Show the diff for a single event or an entire session.
    Diff {
        /// Event ID.
        event_id: Option<u64>,

        /// Show the full diff of an entire session.
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
        /// Event ID to restore to.
        event_id: Option<u64>,

        /// Restore a specific file by path.
        #[arg(long)]
        file: Option<String>,

        /// Roll back an entire agent session atomically.
        #[arg(long)]
        session: Option<String>,
    },

    /// Panic button — undo the last agent action.
    Oops {
        /// Skip the confirmation prompt.
        #[arg(long)]
        confirm: bool,
    },

    /// Pin the current state so it's never garbage collected.
    Pin {
        label: String,
    },

    /// Show per-line agent attribution for a file (v2).
    Blame {
        file: String,
    },

    /// Interactive timeline browser.
    Tui,

    /// Run a command and attribute all its file writes to a specific agent.
    Exec {
        /// Agent identifier for attribution.
        #[arg(long)]
        agent: String,

        /// Optional session label.
        #[arg(long)]
        label: Option<String>,

        /// Command and arguments.
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
    /// Start a new session. Prints the session ID.
    Start {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        metadata: Option<String>,
    },
    /// End a session.
    End { session_id: String },
}

#[derive(Subcommand)]
enum HookCmd {
    /// PreToolUse hook — reads JSON on stdin.
    Pre,
    /// PostToolUse hook — reads JSON on stdin.
    Post,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "agent_undo=info".into()),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Init => println!("agent-undo init: not yet implemented"),
        Command::Status => println!("agent-undo status: not yet implemented"),
        Command::Serve => println!("agent-undo serve: not yet implemented"),
        Command::Log { .. } => println!("agent-undo log: not yet implemented"),
        Command::Sessions => println!("agent-undo sessions: not yet implemented"),
        Command::Diff { .. } => println!("agent-undo diff: not yet implemented"),
        Command::Show { .. } => println!("agent-undo show: not yet implemented"),
        Command::Restore { .. } => println!("agent-undo restore: not yet implemented"),
        Command::Oops { .. } => println!("agent-undo oops: not yet implemented"),
        Command::Pin { .. } => println!("agent-undo pin: not yet implemented"),
        Command::Blame { .. } => println!("agent-undo blame: v2 feature"),
        Command::Tui => println!("agent-undo tui: not yet implemented"),
        Command::Exec { .. } => println!("agent-undo exec: not yet implemented"),
        Command::Session(_) => println!("agent-undo session: not yet implemented"),
        Command::Hook(_) => println!("agent-undo hook: not yet implemented"),
        Command::Gc => println!("agent-undo gc: not yet implemented"),
    }

    Ok(())
}
