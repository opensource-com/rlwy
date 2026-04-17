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
    Ls,
    /// Watch the active deployment of a service
    Watch {
        /// Service id. If omitted, you'll be asked to pick one
        service_id: Option<String>,
        /// Poll interval in seconds
        #[arg(long, default_value_t = 3)]
        interval: u64,
    },
    /// Print build + deploy logs for a specific deployment
    Logs {
        deployment_id: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env from CWD (for local testing); missing file is fine.
    let _ = dotenvy::dotenv();

    let cli = Cli::parse();

    match cli.command {
        Cmd::Login { token } => commands::login::run(token).await,
        Cmd::Whoami => commands::login::whoami().await,
        Cmd::Ls => commands::list::run().await,
        Cmd::Watch { service_id, interval } => commands::watch::run(service_id, interval).await,
        Cmd::Logs { deployment_id } => commands::watch::logs(deployment_id).await,
    }
}
