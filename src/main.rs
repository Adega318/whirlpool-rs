mod app;
mod exchange;
mod types;

use anyhow::Result;
use clap::Parser;
use tokio::sync::mpsc;
use tracing::debug;
use tracing_subscriber::EnvFilter;

use crate::app::App;
use crate::exchange::kraken::Kraken;
use crate::exchange::traits::Exchange;
use crate::exchange::websocket::WsClient;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long, help = "Run without the terminal UI")]
    no_tui: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    init_logging();

    let (updates_tx, mut updates_rx) = mpsc::channel(32);

    let client = WsClient::new(Kraken, 32);
    let commands_tx = client.send_tx.clone();
    let shutdown_tx = client.shutdown_tx.clone();

    let ws = tokio::spawn(client.run(updates_tx));

    Kraken
        .subscribe(&commands_tx, vec!["BTC/USD".to_string()])
        .await?;

    let reader = tokio::spawn(async move {
        while let Some(update) = updates_rx.recv().await {
            debug!("{update}");
        }
        Ok::<_, anyhow::Error>(())
    });

    if !args.no_tui {
        let mut app = App::new();
        tokio::task::spawn_blocking(move || app.run()).await??;
    } else {
        tokio::signal::ctrl_c().await?;
    }

    shutdown_tx.send(()).await?;

    ws.await??;

    reader.await??;

    Ok(())
}

fn init_logging() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();
}
