use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, KeyEventKind};
use std::time::Duration;

/// Terminal event types
pub enum Event {
    /// Key press event
    Key(KeyEvent),
    /// Tick event (for periodic updates)
    Tick,
}

/// Event handler with tick-based polling
pub struct EventHandler {
    tick_rate: Duration,
}

impl EventHandler {
    /// Create a new event handler with the specified tick rate
    pub fn new(tick_rate: Duration) -> Self {
        Self { tick_rate }
    }

    /// Wait for the next event
    pub fn next(&self) -> std::io::Result<Event> {
        // Poll for events with timeout
        if event::poll(self.tick_rate)? {
            // There's an event available
            match event::read()? {
                CrosstermEvent::Key(key) => {
                    // Only process key press events (not release)
                    if key.kind == KeyEventKind::Press {
                        Ok(Event::Key(key))
                    } else {
                        // If it's a release event, wait for the next event
                        self.next()
                    }
                }
                _ => self.next(), // Ignore other events and wait for next
            }
        } else {
            // Timeout occurred, send tick
            Ok(Event::Tick)
        }
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new(Duration::from_millis(250))
    }
}
