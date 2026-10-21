use anyhow::Result;
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::types::Update;

#[async_trait]
pub trait Exchange: Send + Sync {
    fn name(&self) -> &'static str;
    fn url(&self) -> &'static str;
    fn subscription(&self) -> String;

    async fn handle_message(&self, text: &str, tx: &mpsc::Sender<Update>) -> Result<()>;
}

pub async fn connect(exchange: &dyn Exchange, tx: mpsc::Sender<Update>) -> Result<()> {
    let (ws, response) = connect_async(exchange.url()).await?;
    println!("{} connected: {}", exchange.name(), response.status());

    let (mut write, mut read) = ws.split();

    write
        .send(Message::Text(exchange.subscription().into()))
        .await?;

    while let Some(message) = read.next().await {
        match message? {
            Message::Text(text) => {
                exchange.handle_message(&text, &tx).await?;
            }
            Message::Close(frame) => {
                println!("{} closed: {frame:?}", exchange.name());
                break;
            }
            _ => {}
        }
    }

    Ok(())
}
