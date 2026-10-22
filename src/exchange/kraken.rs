use anyhow::{Context, Result};
use chrono::DateTime;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::mpsc;

use crate::{exchange::traits::Exchange, types::Update};

const BOOK_CHANNEL: &str = "book";

#[derive(Debug, Deserialize)]
pub struct KrakenBookMessage {
    #[serde(default)]
    pub data: Vec<KrakenBookData>,
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

impl Kraken {
    fn parse_book(book: KrakenBookData) -> Result<Option<Update>> {
        let (Some(symbol), Some(timestamp)) = (book.symbol, book.timestamp) else {
            return Ok(None);
        };

        let timestamp = DateTime::parse_from_rfc3339(&timestamp)
            .context("Invalid Kraken book timestamp")?
            .timestamp_millis();

        let timestamp =
            u64::try_from(timestamp).context("Kraken timestamp is before the Unix epoch")?;

        Ok(Some(Update {
            exchange: Self::NAME,
            symbol,
            timestamp,
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
        }))
    }
}

impl Exchange for Kraken {
    const NAME: &'static str = "kraken";
    const URL: &'static str = "wss://ws.kraken.com/v2";

    async fn subscribe(&self, tx: &mpsc::Sender<String>, symbols: Vec<String>) -> Result<()> {
        let subscription = json!({
            "method": "subscribe",
            "params": {
                "channel": BOOK_CHANNEL,
                "symbol": symbols,
            }
        })
        .to_string();

        tx.send(subscription).await?;
        Ok(())
    }

    async fn unsubscribe(&self, tx: &mpsc::Sender<String>, symbols: Vec<String>) -> Result<()> {
        let unsubscription = json!({
            "method": "unsubscribe",
            "params": {
                "channel": BOOK_CHANNEL,
                "symbol": symbols
            }
        })
        .to_string();

        tx.send(unsubscription).await?;
        Ok(())
    }

    async fn handle_message(&self, message: &str) -> Result<Vec<Update>> {
        // Ignore acknowledgements and unrelated channels before parsing
        // the exchange-specific book structure.
        let value: Value = serde_json::from_str(message)?;

        if value.get("channel").and_then(Value::as_str) != Some(BOOK_CHANNEL) {
            return Ok(Vec::new());
        }

        let message: KrakenBookMessage =
            serde_json::from_value(value).context("Failed to deserialize Kraken book message")?;

        message
            .data
            .into_iter()
            .map(Self::parse_book)
            .collect::<Result<Vec<_>>>()
            .map(|updates| updates.into_iter().flatten().collect())
    }
}
