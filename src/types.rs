use std::fmt;

use crate::utils::fmt_num;

#[derive(Debug)]
pub struct Update {
    pub exchange: &'static str,
    pub exchanges_status: bool,
    pub symbol: String,
    pub timestamp: u64,
    pub bids: Vec<(f64, f64)>,
    pub asks: Vec<(f64, f64)>,
}

impl fmt::Display for Update {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Exchange: {}", self.exchange)?;
        writeln!(f, "Symbol:   {}", self.symbol)?;
        writeln!(f, "Time:     {}", self.timestamp)?;

        writeln!(f, "Bids:")?;
        for (price, qty) in &self.bids {
            writeln!(f, "  Price: {}, Qty: {}", fmt_num(*price), fmt_num(*qty))?;
        }

        writeln!(f, "Asks:")?;
        for (price, qty) in &self.asks {
            writeln!(f, "  Price: {}, Qty: {}", fmt_num(*price), fmt_num(*qty))?;
        }

        Ok(())
    }
}
