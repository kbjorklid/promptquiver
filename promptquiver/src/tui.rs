use anyhow::Result;
use crossterm::{
    cursor,
    event::{self, Event},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::{io, panic, time::Duration};

#[derive(Debug)]
pub struct Tui<B: Backend> {
    pub terminal: Terminal<B>,
}

impl<B: Backend> Tui<B> {
    pub const fn new(terminal: Terminal<B>) -> Self {
        Self { terminal }
    }

    /// Enters the terminal's alternate screen and enables raw mode.
    ///
    /// # Errors
    /// Returns an error if the terminal cannot be initialized.
    ///
    /// # Panics
    /// Panics if the panic hook fails to reset the terminal.
    pub fn enter(&mut self) -> Result<()>
    where
        B::Error: Send + Sync + 'static,
    {
        terminal::enable_raw_mode()?;
        execute!(
            io::stdout(),
            EnterAlternateScreen,
            cursor::Hide,
            event::EnableBracketedPaste,
            event::EnableMouseCapture,
        )?;
        let panic_hook = panic::take_hook();
        panic::set_hook(Box::new(move |panic| {
            Self::reset().expect("failed to reset terminal");
            panic_hook(panic);
        }));
        self.terminal.hide_cursor()?;
        self.terminal.clear()?;
        Ok(())
    }

    /// Resets the terminal to its original state.
    ///
    /// # Errors
    /// Returns an error if raw mode cannot be disabled or the alternate screen cannot be left.
    pub fn reset() -> Result<()> {
        terminal::disable_raw_mode()?;
        execute!(
            io::stdout(),
            LeaveAlternateScreen,
            cursor::Show,
            event::DisableBracketedPaste,
            event::DisableMouseCapture,
        )?;
        Ok(())
    }

    /// Exits the terminal's alternate screen and disables raw mode.
    ///
    /// # Errors
    /// Returns an error if the terminal cannot be reset.
    pub fn exit(&mut self) -> Result<()>
    where
        B::Error: Send + Sync + 'static,
    {
        Self::reset()?;
        self.terminal.show_cursor()?;
        Ok(())
    }
}

/// Waits for the next terminal event.
///
/// # Errors
/// Returns an error if event polling fails.
pub fn next_event(timeout: Duration) -> Result<Option<Event>> {
    if event::poll(timeout)? {
        Ok(Some(event::read()?))
    } else {
        Ok(None)
    }
}
