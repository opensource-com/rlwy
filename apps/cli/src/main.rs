mod api;
mod commands;
mod config;
mod ui;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "rlwy",
    version,
    about = "Watch Railway deployments from your terminal",
    propagate_version = true
)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Save a Railway API token
    Login {
        /// Pass token inline instead of prompting
        #[arg(long)]
        token: Option<String>,
    },
    /// Show the account the stored token belongs to
    Whoami,
    /// List projects, services, and the latest deployment of each
    Ls {
        /// Optional project-name filter (case-insensitive substring). `rlwy ls uft` shows only projects whose name contains "uft"
        query: Option<String>,
    },
    /// Watch the active deployment of a service
    Watch {
        /// Service id, name, or `project/service`. Omit to use the last choice
        query: Option<String>,
        /// Poll interval in seconds
        #[arg(long, default_value_t = 3)]
        interval: u64,
        /// Always open the picker, even if a last service is remembered
        #[arg(long)]
        pick: bool,
    },
    /// Print build + deploy logs for a specific deployment
    Logs {
        deployment_id: String,
    },
    /// Download and install the latest rlwy release
    Upgrade,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env from CWD (for local testing); missing file is fine.
    let _ = dotenvy::dotenv();

    let cli = Cli::parse();

    match cli.command {
        Cmd::Login { token } => commands::login::run(token).await,
        Cmd::Whoami => commands::login::whoami().await,
        Cmd::Ls { query } => commands::list::run(query).await,
        Cmd::Watch { query, interval, pick } => commands::watch::run(query, interval, pick).await,
        Cmd::Logs { deployment_id } => commands::watch::logs(deployment_id).await,
        Cmd::Upgrade => commands::upgrade::run().await,
    }
}
