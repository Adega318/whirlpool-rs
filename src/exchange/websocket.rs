use crate::exchange::traits::Exchange;
use crate::types::Update;
use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, info};

pub struct WsClient {
    pub send_tx: mpsc::Sender<String>,
    send_rx: mpsc::Receiver<String>,
    pub shutdown_tx: mpsc::Sender<()>,
    shutdown_rx: mpsc::Receiver<()>,
    pub exchange: Box<dyn Exchange>,
}

impl WsClient {
    pub fn new(exchange: Box<dyn Exchange>, capacity: usize) -> Self {
        let (send_tx, send_rx) = mpsc::channel(capacity);
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

        Self {
            send_tx,
            send_rx,
            shutdown_tx,
            shutdown_rx,
            exchange,
        }
    }

    pub async fn run(mut self, updates_tx: mpsc::Sender<Update>) -> Result<()> {
        let (ws, _) = connect_async(self.exchange.url())
            .await
            .with_context(|| format!("Failed to connect to {}", self.exchange.name()))?;

        info!("{} connected", self.exchange.name());

        let (mut writer, mut reader) = ws.split();

        loop {
            tokio::select! {
                // Shutdown messages
                _ = self.shutdown_rx.recv() => {
                    break;
                }

                // Send outgoing commands.
                command = self.send_rx.recv() => {
                    match command {
                        Some(command) => {
                            writer
                                .send(Message::Text(command.into()))
                                .await
                                .context("Failed to send WebSocket message")?;
                        }
                        None => break,
                    }
                }

                // Read incoming messages.
                message = reader.next() => {
                    match message {
                        Some(Ok(Message::Text(text))) => {
                            let updates = self.exchange
                                .handle_message(&text)
                                .await?;

                            for update in updates {
                                debug!("{}",update);
                                updates_tx.send(update).await.context("Update consumer channel closed")?;
                            }
                        }

                        Some(Ok(Message::Ping(payload))) => {
                            writer
                                .send(Message::Pong(payload))
                                .await?;
                        }

                        Some(Ok(Message::Close(frame))) => {
                            info!("{} disconnected: {:?}", self.exchange.name(), frame);
                            break;
                        }

                        Some(Ok(_)) => {}

                        Some(Err(err)) => {
                            return Err(err).context("WebSocket read failed");
                        }

                        None => break,
                    }
                }
            }
        }

        let _ = writer.close().await;
        info!("{} stopped", self.exchange.name());

        Ok(())
    }
}
