#[derive(Debug)]
pub struct Update {
    pub exchange: &'static str,
    pub symbol: String,
    pub time: u64,
    pub bids: Vec<(f64, f64)>,
    pub asks: Vec<(f64, f64)>,
}
