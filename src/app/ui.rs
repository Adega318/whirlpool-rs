use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Paragraph, Row, Table},
};

use crate::agregate::{Aggregate, ExchangeData, Stats};

use super::App;

pub fn render(_app: &mut App, frame: &mut Frame, agg: &Aggregate) {
    let [top, bottom] = Layout::vertical([
        Constraint::Length(agg.best_sym.len() as u16 + 3), // rows + header + borders
        Constraint::Min(0),
    ])
    .areas(frame.area());

    frame.render_widget(
        best_table(&agg.best_sym, " Best across connected exchanges "),
        top,
    );

    let n = agg.exchanges.len() as u32;
    let cols =
        Layout::horizontal(vec![Constraint::Ratio(1, n.max(1)); agg.exchanges.len()]).split(bottom);
    for (ex, &area) in agg.exchanges.iter().zip(cols.iter()) {
        frame.render_widget(exchange_view(ex), area);
    }
}

fn best_table(stats: &[Stats], title: &str) -> Table<'static> {
    let rows = stats.iter().map(|s| {
        // negative spread = crossed market
        let spread_color = if s.spread < 0.0 {
            Color::Red
        } else {
            Color::Yellow
        };
        Row::new([
            s.symbol.clone().bold(),
            s.best_bid.to_string().green(),
            s.best_ask.to_string().red(),
            format!("{:.2}", s.spread).fg(spread_color),
        ])
    });
    Table::new(
        rows,
        [
            Constraint::Length(14),
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Length(10),
        ],
    )
    .header(
        Row::new(["Symbol", "Best bid", "Best ask", "Spread"])
            .style(Style::new().add_modifier(Modifier::BOLD | Modifier::UNDERLINED)),
    )
    .block(Block::bordered().title(title.to_string()))
}

fn exchange_view(ex: &ExchangeData) -> Paragraph<'static> {
    let (label, color) = if ex.connected {
        ("up", Color::Green)
    } else {
        ("down", Color::Red)
    };
    let title = Line::from(vec![
        Span::raw(format!(" {} ", ex.exchange)).bold(),
        Span::styled(format!("● {label} "), Style::new().fg(color)),
    ]);

    let mut lines: Vec<Line> = ex
        .best_sym
        .iter()
        .map(|s| Line::from(s.to_string()).dim())
        .collect();

    for b in &ex.books {
        lines.push(Line::raw(""));
        lines.push(Line::from(b.symbol.clone()).bold().underlined());
        // asks high -> low, then bids high -> low, so the spread sits in the middle
        lines.extend(
            b.asks
                .iter()
                .rev()
                .map(|l| Line::from(format!("  {l}")).red()),
        );
        lines.extend(b.bids.iter().map(|l| Line::from(format!("  {l}")).green()));
    }

    Paragraph::new(lines).block(Block::bordered().title(title))
}
