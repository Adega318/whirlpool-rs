use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{exchange::traits::Exchange, types::Update};

const DEPTH_EVENT: &str = "depthUpdate";

#[derive(Debug, Deserialize)]
pub struct BinanceDepthEvent {
    /// Event time (ms since epoch)
    #[serde(rename = "E")]
    pub event_time: u64,
    /// Symbol, uppercase e.g. "BTCUSDT"
    #[serde(rename = "s")]
    pub symbol: String,
    /// First update ID in event
    #[expect(dead_code, reason = "sequence validation not yet implemented")]
    #[serde(rename = "U")]
    pub first_update_id: u64,
    /// Final update ID in event
    #[expect(dead_code, reason = "sequence validation not yet implemented")]
    #[serde(rename = "u")]
    pub final_update_id: u64,
    #[serde(rename = "b", default)]
    pub bids: Vec<[String; 2]>,
    #[serde(rename = "a", default)]
    pub asks: Vec<[String; 2]>,
}

pub struct Binance {
    id: String,
}

impl Default for Binance {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().simple().to_string(),
        }
    }
}

impl Binance {
    fn name(&self) -> &'static str {
        "binance"
    }

    fn url(&self) -> &'static str {
        "wss://stream.binance.com:9443/ws"
    }

    fn parse_levels(levels: Vec<[String; 2]>) -> Result<Vec<(f64, f64)>> {
        levels
            .into_iter()
            .map(|[price, qty]| {
                Ok((
                    price.parse().context("Invalid Binance price")?,
                    qty.parse().context("Invalid Binance quantity")?,
                ))
            })
            .collect()
    }

    fn stream_names(symbols: &[String]) -> Vec<String> {
        symbols
            .iter()
            .map(|s| format!("{}@depth@100ms", s.replace("/", "").to_lowercase()))
            .collect()
    }
}

#[async_trait]
impl Exchange for Binance {
    fn name(&self) -> &'static str {
        self.name()
    }

    fn url(&self) -> &'static str {
        self.url()
    }

    async fn subscribe(&self, tx: &mpsc::Sender<String>, symbols: &[String]) -> Result<()> {
        let msg = json!({
            "method": "SUBSCRIBE",
            "params": Self::stream_names(symbols),
            "id": self.id
        })
        .to_string();

        tx.send(msg).await?;
        Ok(())
    }

    async fn unsubscribe(&self, tx: &mpsc::Sender<String>, symbols: &[String]) -> Result<()> {
        let msg = json!({
            "method": "UNSUBSCRIBE",
            "params": Self::stream_names(symbols),
            "id": self.id
        })
        .to_string();

        tx.send(msg).await?;
        Ok(())
    }

    async fn handle_message(&self, message: &str) -> Result<Vec<Update>> {
        let value: Value = serde_json::from_str(message)?;

        // Acks look like {"result":null,"id":1}; they have no "e" field.
        if value.get("e").and_then(Value::as_str) != Some(DEPTH_EVENT) {
            return Ok(Vec::new());
        }

        let event: BinanceDepthEvent =
            serde_json::from_value(value).context("Failed to deserialize Binance depth event")?;

        Ok(vec![Update {
            exchange: self.name(),
            exchanges_status: true,
            symbol: event.symbol,
            timestamp: event.event_time,
            bids: Self::parse_levels(event.bids)?,
            asks: Self::parse_levels(event.asks)?,
        }])
    }
}
