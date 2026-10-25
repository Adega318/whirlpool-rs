mod app;
mod exchange;
mod types;

use crate::app::App;
use crate::exchange::kraken::Kraken;
use crate::exchange::traits::Exchange;
use crate::exchange::websocket::WsClient;
use anyhow::Result;
use clap::Parser;
use tokio::sync::mpsc;
use tracing::debug;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long, help = "Run without the terminal UI")]
    no_tui: bool,
}

const CAPACITY: usize = 32;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    init_logging();

    let (updates_tx, mut updates_rx) = mpsc::channel(CAPACITY);
    let clients = vec![WsClient::new(Kraken, CAPACITY)];
    let symbols = vec!["BTC/USD".to_string()];

    let mut shutdown_txs = Vec::new();
    let mut ws_handles = Vec::new();

    for client in clients {
        shutdown_txs.push(client.shutdown_tx.clone());
        client.exchange.subscribe(&client.send_tx, &symbols).await?;
        ws_handles.push(tokio::spawn(client.run(updates_tx.clone())));
    }
    drop(updates_tx);

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

    for tx in &shutdown_txs {
        let _ = tx.send(()).await;
    }

    for ws in ws_handles {
        ws.await??;
    }

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
