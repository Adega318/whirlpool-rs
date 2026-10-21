mod exchange;
mod types;

use anyhow::Result;

use tokio::sync::mpsc;

use crate::exchange::websocket::connect;
use exchange::kraken::Kraken;

use self::exchange::websocket::Exchange;

#[tokio::main]
async fn main() -> Result<()> {
    let (tx, mut rx) = mpsc::channel::<types::Update>(1000);

    let consumer = tokio::spawn(async move {
        while let Some(update) = rx.recv().await {
            println!("Exchange: {}", update.exchange);
            println!("Symbol: {}", update.symbol);
            println!("Market update: {update:?}");
        }
    });

    let exchanges: Vec<Box<dyn Exchange>> = vec![Box::new(Kraken)];
    let mut handles = Vec::new();

    for exchange in exchanges {
        let tx = tx.clone();

        let handle = tokio::spawn(async move {
            if let Err(err) = connect(exchange.as_ref(), tx).await {
                eprintln!("Exchange error: {err:#}");
            }
        });

        handles.push(handle);
    }

    drop(tx);

    for handle in handles {
        handle.await?;
    }

    consumer.await?;

    Ok(())
}
