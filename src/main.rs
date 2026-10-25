mod agregate;
mod app;
mod exchange;
mod types;

use crate::app::App;
use crate::exchange::kraken::Kraken;
use crate::exchange::websocket::WsClient;
use anyhow::Result;
use clap::Parser;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{mpsc, watch};
use tracing_subscriber::{
    EnvFilter, Layer, filter::FilterFn, layer::SubscriberExt, util::SubscriberInitExt,
};

use self::agregate::Aggregate;
use self::exchange::binance::Binance;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long, help = "Run without the terminal UI")]
    no_tui: bool,
}

const CAPACITY: usize = 32;

static LOGGING_ENABLED: AtomicBool = AtomicBool::new(true);

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    init_logging();

    let (updates_tx, updates_rx) = mpsc::channel(CAPACITY);
    let clients = vec![
        WsClient::new(Box::new(Kraken), CAPACITY),
        WsClient::new(Box::new(Binance::default()), CAPACITY),
    ];
    let symbols = vec!["BTC/USDT".to_string(), "ETH/USDT".to_string()];

    let mut shutdown_txs = Vec::new();
    let mut ws_handles = Vec::new();

    for client in clients {
        shutdown_txs.push(client.shutdown_tx.clone());
        client.exchange.subscribe(&client.send_tx, &symbols).await?;
        ws_handles.push(tokio::spawn(client.run(updates_tx.clone())));
    }
    drop(updates_tx);

    let (state_tx, state_rx) = watch::channel(Aggregate::default());
    tokio::spawn(Aggregate::run(state_tx, updates_rx));

    if !args.no_tui {
        let mut app = App::new(state_rx);
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

    Ok(())
}

fn init_logging() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let filter = FilterFn::new(move |_meta| {
        if !LOGGING_ENABLED.load(Ordering::Relaxed) {
            return false;
        }
        true
    });
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_filter(filter)
        .with_filter(env_filter);
    tracing_subscriber::registry().with(fmt_layer).init();
}

fn set_logging_enabled(enabled: bool) {
    LOGGING_ENABLED.store(enabled, Ordering::Relaxed);
}
