use anyhow::Result;
use tokio::sync::mpsc;

use crate::types::Update;

pub trait Exchange {
    const NAME: &'static str;
    const URL: &'static str;

    async fn handle_message(&self, message: &str) -> Result<Vec<Update>>;

    async fn subscribe(&self, tx: &mpsc::Sender<String>, symbols: &Vec<String>) -> Result<()>;
    async fn unsubscribe(&self, tx: &mpsc::Sender<String>, symbols: &Vec<String>) -> Result<()>;
}
