use anyhow::Result;
use ratatui::prelude::*;
use std::{io, panic, time::Duration};
use crossterm::{
    cursor,
    event::{self, Event},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};

#[derive(Debug)]
pub struct Tui<B: Backend> {
    pub terminal: Terminal<B>,
}

impl<B: Backend> Tui<B> {
    pub fn new(terminal: Terminal<B>) -> Self {
        Self { terminal }
    }

    pub fn enter(&mut self) -> Result<()>
    where
        B::Error: Send + Sync + 'static,
    {
        terminal::enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen, cursor::Hide)?;
        let panic_hook = panic::take_hook();
        panic::set_hook(Box::new(move |panic| {
            Self::reset().expect("failed to reset terminal");
            panic_hook(panic);
        }));
        self.terminal.hide_cursor()?;
        self.terminal.clear()?;
        Ok(())
    }

    pub fn reset() -> Result<()> {
        terminal::disable_raw_mode()?;
        execute!(io::stdout(), LeaveAlternateScreen, cursor::Show)?;
        Ok(())
    }

    pub fn exit(&mut self) -> Result<()>
    where
        B::Error: Send + Sync + 'static,
    {
        Self::reset()?;
        self.terminal.show_cursor()?;
        Ok(())
    }
}

pub fn next_event(timeout: Duration) -> Result<Option<Event>> {
    if event::poll(timeout)? {
        Ok(Some(event::read()?))
    } else {
        Ok(None)
    }
}
