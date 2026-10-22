use std::fmt;

#[derive(Debug)]
pub struct Update {
    pub exchange: &'static str,
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
            writeln!(f, "  Price: {price:.2}, Qty: {qty:.6}")?;
        }

        writeln!(f, "Asks:")?;
        for (price, qty) in &self.asks {
            writeln!(f, "  Price: {price:.2}, Qty: {qty:.6}")?;
        }

        Ok(())
    }
}
