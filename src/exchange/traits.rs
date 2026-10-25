use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::types::Update;

#[async_trait]
pub trait Exchange: Send + Sync {
    fn name(&self) -> &'static str;
    fn url(&self) -> &'static str;

    async fn handle_message(&self, message: &str) -> Result<Vec<Update>>;

    async fn subscribe(&self, tx: &mpsc::Sender<String>, symbols: &[String]) -> Result<()>;
    async fn unsubscribe(&self, tx: &mpsc::Sender<String>, symbols: &[String]) -> Result<()>;
}
