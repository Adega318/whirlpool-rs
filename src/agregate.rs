use std::collections::{BTreeMap, HashMap};
use std::fmt::{self, Display};

use tokio::sync::{mpsc::Receiver, watch};

use crate::types::Update;
use crate::utils::fmt_num;

const DEPTH: usize = 10;
type Symbol = String;

fn each<T: Display>(f: &mut fmt::Formatter, items: &[T], pre: &str) -> fmt::Result {
    items.iter().try_for_each(|i| writeln!(f, "{pre}{i}"))
}

#[derive(Clone, Copy, Debug)]
pub struct PriceLevel {
    price: f64,
    qty: f64,
}
impl Display for PriceLevel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} @ {}", fmt_num(self.price), fmt_num(self.qty))
    }
}

#[derive(Clone, Debug)]
pub struct SymbolBook {
    pub symbol: Symbol,
    pub bids: Vec<PriceLevel>,
    pub asks: Vec<PriceLevel>,
}
impl Display for SymbolBook {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}\nBids:", self.symbol)?;
        each(f, &self.bids, "  ")?;
        writeln!(f, "Asks:")?;
        each(f, &self.asks, "  ")
    }
}

#[derive(Clone, Debug)]
pub struct Stats {
    pub symbol: Symbol,
    pub spread: f64,
    pub best_bid: PriceLevel,
    pub best_ask: PriceLevel,
}
impl Stats {
    fn new(symbol: Symbol, best_bid: PriceLevel, best_ask: PriceLevel) -> Self {
        Self {
            symbol,
            spread: best_ask.price - best_bid.price,
            best_bid,
            best_ask,
        }
    }
}
impl Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}, spread: {}, best_bid: {}, best_ask: {}",
            self.symbol,
            fmt_num(self.spread),
            self.best_bid,
            self.best_ask
        )
    }
}

#[derive(Clone, Debug)]
pub struct ExchangeData {
    pub exchange: &'static str,
    pub connected: bool,
    pub best_sym: Vec<Stats>,
    pub books: Vec<SymbolBook>,
}
impl Display for ExchangeData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "{} - {}",
            self.exchange,
            if self.connected { "up" } else { "down" }
        )?;
        each(f, &self.best_sym, "")?;
        each(f, &self.books, "")
    }
}

#[derive(Clone, Debug, Default)]
pub struct Aggregate {
    pub best_sym: Vec<Stats>,
    pub exchanges: Vec<ExchangeData>,
}
impl Display for Aggregate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Aggregate:")?;
        each(f, &self.best_sym, "")?;
        each(f, &self.exchanges, "")
    }
}

// ---- internal state -------------------------------------------------------

/// Keys are `f64::to_bits()`: for positive finite floats this preserves price order.
#[derive(Default)]
struct OrderBook {
    bids: BTreeMap<u64, f64>,
    asks: BTreeMap<u64, f64>,
    last_ts: u64,
}

fn lvl((k, q): (&u64, &f64)) -> PriceLevel {
    PriceLevel {
        price: f64::from_bits(*k),
        qty: *q,
    }
}

fn apply_side(side: &mut BTreeMap<u64, f64>, levels: &[(f64, f64)]) {
    for &(p, q) in levels {
        if p.is_finite() && p > 0.0 && q.is_finite() && q >= 0.0 {
            if q == 0.0 {
                side.remove(&p.to_bits());
            } else {
                side.insert(p.to_bits(), q);
            }
        }
    }
}

impl OrderBook {
    /// Returns false if the update was stale.
    fn apply(&mut self, bids: &[(f64, f64)], asks: &[(f64, f64)], ts: u64) -> bool {
        if ts < self.last_ts {
            return false;
        }
        self.last_ts = ts;
        apply_side(&mut self.bids, bids);
        apply_side(&mut self.asks, asks);
        true
    }
    fn best(&self) -> (Option<PriceLevel>, Option<PriceLevel>) {
        (
            self.bids.iter().next_back().map(lvl),
            self.asks.iter().next().map(lvl),
        )
    }
    fn snapshot(&self, symbol: &Symbol) -> SymbolBook {
        SymbolBook {
            symbol: symbol.clone(),
            bids: self.bids.iter().rev().take(DEPTH).map(lvl).collect(),
            asks: self.asks.iter().take(DEPTH).map(lvl).collect(),
        }
    }
}

#[derive(Default)]
struct ExchangeState {
    connected: bool,
    books: BTreeMap<Symbol, OrderBook>,
}

// ---- aggregation ----------------------------------------------------------

impl Aggregate {
    pub async fn run(state_tx: watch::Sender<Aggregate>, mut rx: Receiver<Update>) {
        let mut exs: BTreeMap<&'static str, ExchangeState> = BTreeMap::new();

        while let Some(u) = rx.recv().await {
            let ex = exs.entry(u.exchange).or_default();
            ex.connected = u.exchanges_status;

            if !ex.connected {
                ex.books.clear(); // don't leak stale quotes
            } else if !ex
                .books
                .entry(u.symbol)
                .or_default()
                .apply(&u.bids, &u.asks, u.timestamp)
            {
                continue; // stale update
            }

            if state_tx.send(Self::build(&exs)).is_err() {
                break;
            }
        }
    }

    fn build(exs: &BTreeMap<&'static str, ExchangeState>) -> Aggregate {
        let mut best: HashMap<&Symbol, (Option<PriceLevel>, Option<PriceLevel>)> = HashMap::new();
        let mut exchanges = Vec::with_capacity(exs.len());

        for (&exchange, st) in exs {
            let (mut best_sym, mut books) = (Vec::new(), Vec::new());
            for (sym, book) in &st.books {
                let (bid, ask) = book.best();
                if let (Some(b), Some(a)) = (bid, ask) {
                    best_sym.push(Stats::new(sym.clone(), b, a));
                }
                books.push(book.snapshot(sym));

                if st.connected {
                    let e = best.entry(sym).or_default();
                    if let Some(b) = bid {
                        if e.0.map_or(true, |c| b.price > c.price) {
                            e.0 = Some(b);
                        }
                    }
                    if let Some(a) = ask {
                        if e.1.map_or(true, |c| a.price < c.price) {
                            e.1 = Some(a);
                        }
                    }
                }
            }
            exchanges.push(ExchangeData {
                exchange,
                connected: st.connected,
                best_sym,
                books,
            });
        }

        let mut best_sym: Vec<Stats> = best
            .into_iter()
            .filter_map(|(s, (b, a))| Some(Stats::new(s.clone(), b?, a?)))
            .collect();
        best_sym.sort_by(|a, b| a.symbol.cmp(&b.symbol)); // stable output order

        Aggregate {
            best_sym,
            exchanges,
        }
    }
}
