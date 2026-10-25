use std::collections::HashMap;
use std::fmt;

use tokio::sync::mpsc::Receiver;
use tokio::sync::watch;
use tracing::debug;

use crate::types::Update;

/// Must match the depth you subscribed with (Kraken minimum is 10).
const DEPTH: usize = 70;

#[derive(Debug, Clone, Copy)]
pub struct PriceLevel {
    pub price: f64,
    pub qty: f64,
}

impl fmt::Display for PriceLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.2} @ {:.4}", self.price, self.qty)
    }
}

#[derive(Debug, Clone)]
pub struct ExchangeStats {
    pub exchange: &'static str,
    pub connected: bool,
    pub best_bid: Option<PriceLevel>,
    pub best_ask: Option<PriceLevel>,
}

fn fmt_level(l: &Option<PriceLevel>) -> String {
    l.map(|l| l.to_string()).unwrap_or_else(|| "-".into())
}

impl fmt::Display for ExchangeStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:<10} {:<5} bid: {:<20} ask: {}",
            self.exchange,
            if self.connected { "up" } else { "down" },
            fmt_level(&self.best_bid),
            fmt_level(&self.best_ask),
        )
    }
}

#[derive(Debug, Clone, Default)]
pub struct Aggregator {
    pub best_bid: Option<PriceLevel>,
    pub best_ask: Option<PriceLevel>,
    pub spread: Option<f64>,
    pub exchange_best: Vec<ExchangeStats>,
}

impl fmt::Display for Aggregator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Best bid: {}", fmt_level(&self.best_bid))?;
        writeln!(f, "Best ask: {}", fmt_level(&self.best_ask))?;
        match self.spread {
            Some(s) => writeln!(f, "Spread:   {s:.2}")?,
            None => writeln!(f, "Spread:   -")?,
        }
        for s in &self.exchange_best {
            writeln!(f, "{s}")?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Bounded per-exchange book (top DEPTH levels per side), private to the task.
// ---------------------------------------------------------------------------

#[derive(Default)]
struct Book {
    bids: Vec<(f64, f64)>, // descending by price, best first
    asks: Vec<(f64, f64)>, // ascending by price, best first
}

fn apply_side(
    side: &mut Vec<(f64, f64)>,
    levels: &[(f64, f64)],
    better: impl Fn(f64, f64) -> bool, // true if a ranks before b
) {
    for &(price, qty) in levels {
        let i = side.partition_point(|&(p, _)| better(p, price));
        let exists = side.get(i).is_some_and(|&(p, _)| p == price);

        match (exists, qty == 0.0) {
            (true, true) => {
                side.remove(i);
            }
            (true, false) => side[i].1 = qty,
            (false, false) => side.insert(i, (price, qty)),
            (false, true) => {}
        }
    }
    side.truncate(DEPTH);
}

impl Book {
    fn apply(&mut self, u: &Update) {
        apply_side(&mut self.bids, &u.bids, |a, b| a > b);
        apply_side(&mut self.asks, &u.asks, |a, b| a < b);
    }

    fn best_bid(&self) -> Option<PriceLevel> {
        self.bids
            .first()
            .map(|&(price, qty)| PriceLevel { price, qty })
    }

    fn best_ask(&self) -> Option<PriceLevel> {
        self.asks
            .first()
            .map(|&(price, qty)| PriceLevel { price, qty })
    }
}

// ---------------------------------------------------------------------------

impl Aggregator {
    pub async fn run(state_tx: watch::Sender<Aggregator>, mut rx: Receiver<Update>) {
        // Books live here, not in the published state, so the TUI never clones them.
        let mut books: HashMap<&'static str, Book> = HashMap::new();

        while let Some(update) = rx.recv().await {
            let exchange = update.exchange;
            if !update.exchanges_status {
                state_tx.send_modify(|agg| agg.mark_disconnected(exchange));
            } else {
                let book = books.entry(exchange).or_default();
                book.apply(&update);
                let (bid, ask) = (book.best_bid(), book.best_ask());

                state_tx.send_modify(|agg| agg.set_exchange_best(exchange, bid, ask));
            }
        }
    }

    pub fn set_exchange_best(
        &mut self,
        exchange: &'static str,
        best_bid: Option<PriceLevel>,
        best_ask: Option<PriceLevel>,
    ) {
        let stats = match self
            .exchange_best
            .iter_mut()
            .find(|s| s.exchange == exchange)
        {
            Some(s) => s,
            None => {
                self.exchange_best.push(ExchangeStats {
                    exchange,
                    connected: true,
                    best_bid: None,
                    best_ask: None,
                });
                self.exchange_best.last_mut().unwrap()
            }
        };

        stats.connected = true;
        stats.best_bid = best_bid;
        stats.best_ask = best_ask;

        self.recompute();
    }

    fn recompute(&mut self) {
        self.best_bid = self
            .exchange_best
            .iter()
            .filter(|s| s.connected)
            .filter_map(|s| s.best_bid)
            .max_by(|a, b| a.price.total_cmp(&b.price));

        self.best_ask = self
            .exchange_best
            .iter()
            .filter(|s| s.connected)
            .filter_map(|s| s.best_ask)
            .min_by(|a, b| a.price.total_cmp(&b.price));

        self.spread = match (self.best_bid, self.best_ask) {
            (Some(b), Some(a)) => Some(a.price - b.price), // negative => crossed book
            _ => None,
        };

        debug!("{self}")
    }

    pub fn mark_disconnected(&mut self, exchange: &'static str) {
        if let Some(s) = self
            .exchange_best
            .iter_mut()
            .find(|s| s.exchange == exchange)
        {
            s.connected = false;
            s.best_bid = None;
            s.best_ask = None;
        }
        self.recompute();
    }
}
