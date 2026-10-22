mod exchange;
mod types;

use anyhow::Result;
use tokio::sync::mpsc;

use crate::exchange::kraken::Kraken;
use crate::exchange::traits::Exchange;
use crate::exchange::websocket::WsClient;

#[tokio::main]
async fn main() -> Result<()> {
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
            println!("{update}");
        }
        Ok::<_, anyhow::Error>(())
    });

    tokio::signal::ctrl_c().await?;

    shutdown_tx.send(()).await?;

    ws.await??;

    reader.await??;

    Ok(())
}
