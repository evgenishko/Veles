mod config;
mod error;
mod extract;
mod fetch;
mod rate_limit;
mod search;
mod server;
mod state;
mod tools;

use clap::Parser;
use rmcp::{ServiceExt, transport::stdio};

use crate::{config::Config, server::VelesServer};

#[derive(Debug, Parser)]
#[command(name = "veles")]
#[command(about = "Local MCP server for controlled web search and page extraction")]
struct Cli {
    /// Run Veles as an MCP server over stdio.
    #[arg(long, default_value_t = true)]
    stdio: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    if !cli.stdio {
        anyhow::bail!("only --stdio transport is supported in this MVP");
    }

    let config = Config::from_env()?;
    let server = VelesServer::new(config)?;
    let running = server.serve(stdio()).await?;
    running.waiting().await?;

    Ok(())
}
