mod event;
mod tui;
mod ui;
mod update;

use anyhow::Result;
use event::{Event, EventHandler};
use ratatui::{Terminal, backend::CrosstermBackend};
use tui::Tui;
use update::update;

#[derive(Debug, Default)]
pub struct App {
    should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self::default()
    }

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
            tui.draw(&mut *self)?;
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
