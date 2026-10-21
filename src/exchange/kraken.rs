use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use tokio::sync::mpsc;

use crate::{exchange::websocket::Exchange, types::Update};

#[derive(Debug, Deserialize)]
pub struct KrakenBookMessage {
    pub channel: Option<String>,
    #[serde(default)]
    pub data: Vec<KrakenBookData>,
    #[serde(rename = "type")]
    pub message_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct KrakenBookData {
    #[serde(default)]
    pub asks: Vec<KrakenLevel>,
    #[serde(default)]
    pub bids: Vec<KrakenLevel>,
    pub checksum: Option<u64>,
    pub symbol: Option<String>,
    pub timestamp: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct KrakenLevel {
    pub price: f64,
    pub qty: f64,
}

pub struct Kraken;

#[async_trait]
impl Exchange for Kraken {
    fn name(&self) -> &'static str {
        "kraken"
    }

    fn url(&self) -> &'static str {
        "wss://ws.kraken.com/v2"
    }

    fn subscription(&self) -> String {
        json!({
            "method": "subscribe",
            "params": {
                "channel": "book",
                "symbol": ["BTC/USD"],
            }
        })
        .to_string()
    }

    async fn handle_message(&self, text: &str, tx: &mpsc::Sender<Update>) -> Result<()> {
        let message: KrakenBookMessage = serde_json::from_str(text)?;

        if message.channel.as_deref() != Some("book") {
            return Ok(());
        }

        for book in message.data {
            let (Some(symbol), Some(timestamp)) = (book.symbol, book.timestamp) else {
                continue;
            };

            let time = chrono::DateTime::parse_from_rfc3339(&timestamp)?.timestamp_millis() as u64;

            let update = Update {
                exchange: self.name(),
                symbol,
                time,
                bids: book
                    .bids
                    .into_iter()
                    .map(|level| (level.price, level.qty))
                    .collect(),
                asks: book
                    .asks
                    .into_iter()
                    .map(|level| (level.price, level.qty))
                    .collect(),
            };

            tx.send(update).await?;
        }

        Ok(())
    }
}
