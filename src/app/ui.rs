use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    widgets::{Block, Borders, Paragraph, Row, Table},
};

use crate::agregator::{Aggregator, PriceLevel};

use super::App;

pub fn render(app: &mut App, frame: &mut Frame, agg: &Aggregator) {
    let [top, bottom] =
        Layout::vertical([Constraint::Length(5), Constraint::Min(0)]).areas(frame.area());

    let fmt = |l: Option<PriceLevel>| {
        l.map(|l| format!("{:.2} @ {:.4}", l.price, l.qty))
            .unwrap_or_else(|| "-".into())
    };

    let summary = format!(
        "Best bid: {}\nBest ask: {}\nSpread:   {}",
        fmt(agg.best_bid),
        fmt(agg.best_ask),
        agg.spread
            .map(|s| format!("{s:.2}"))
            .unwrap_or_else(|| "-".into()),
    );
    frame.render_widget(
        Paragraph::new(summary).block(Block::default().borders(Borders::ALL).title("Aggregate")),
        top,
    );

    let rows = agg.exchange_best.iter().map(|s| {
        Row::new(vec![
            s.exchange.to_string(),
            if s.connected { "up" } else { "down" }.to_string(),
            fmt(s.best_bid),
            fmt(s.best_ask),
        ])
    });
    let table = Table::new(
        rows,
        [
            Constraint::Length(12),
            Constraint::Length(6),
            Constraint::Length(22),
            Constraint::Length(22),
        ],
    )
    .header(Row::new(["Exchange", "Conn", "Best bid", "Best ask"]))
    .block(Block::default().borders(Borders::ALL).title("Exchanges"));
    frame.render_widget(table, bottom);
}
