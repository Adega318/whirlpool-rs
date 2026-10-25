mod event;
mod tui;
mod ui;
mod update;

use anyhow::Result;
use event::{Event, EventHandler};
use ratatui::{Terminal, backend::CrosstermBackend};
use tokio::sync::watch;
use tui::Tui;
use update::update;

use crate::agregator::Aggregator;

#[derive(Debug)]
pub struct App {
    should_quit: bool,
    aggregator_rx: watch::Receiver<Aggregator>,
}

impl App {
    pub fn new(rx: watch::Receiver<Aggregator>) -> Self {
        Self {
            should_quit: false,
            aggregator_rx: rx,
        }
    }

    #[expect(dead_code, reason = "yet to use")]
    pub fn tick(&self) {}

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn run(&mut self) -> Result<()> {
        let backend = CrosstermBackend::new(std::io::stderr());
        let terminal = Terminal::new(backend)?;
        let events = EventHandler::new(250);
        let mut tui = Tui::new(terminal, events);
        tui.enter()?;

        while !self.should_quit {
            // Fresh snapshot every iteration; the lock is released right after the clone.
            let snapshot = self.aggregator_rx.borrow_and_update().clone();
            tui.draw(&mut *self, &snapshot)?;

            match tui.events.next()? {
                Event::Tick => {}
                Event::Key(key_event) => update(&mut *self, key_event),
                Event::Mouse(_) => {}
                Event::Resize(_, _) => {}
            };
        }

        tui.exit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {}
