use crate::Color;
use crate::error::Error;
use crate::key_event::{Event, KeyEvent, KeyModifier};
use crate::windows;
use std::sync::mpsc::channel;
use std::thread;
use std::time::Duration;

pub trait Terminal {
    fn read_event() -> Result<Event, Error>;

    fn read_event_timeout() -> Result<Event, Error> {
        let (sender, receiver) = channel();

        thread::spawn(move || sender.send(Self::read_event()).unwrap());

        loop {
            if let Ok(ch) = receiver.recv_timeout(Duration::from_millis(16)) {
                return ch;
            }
        }
    }

    fn alternate_screen_buffer(&mut self) -> Result<(), Error>;

    fn clear_screen(&mut self) -> Result<(), Error>;

    fn enable_raw_mode(&mut self) -> Result<(), Error>;

    fn get_cursor_position(&self) -> Result<(usize, usize), Error>;

    fn get_screen_size(&self) -> Result<(usize, usize), Error>;

    fn scroll_up(&self, height: usize) -> Result<(), Error>;

    fn set_cursor_position(&mut self, x: usize, y: usize) -> Result<(), Error>;

    fn set_text_attribute(&mut self, x: usize, y: usize, length: usize) -> Result<(), Error>;

    fn write(
        &mut self,
        x: usize,
        y: usize,
        row: &[char],
        color: Color,
        rev: bool,
    ) -> Result<(), Error>;
}

// -----------------------------------------------------------------------------------------------

pub struct WindowsCon;

impl Terminal for WindowsCon {
    fn read_event() -> Result<Event, Error> {
        windows::read_event()
    }

    fn alternate_screen_buffer(&mut self) -> Result<(), Error> {
        windows::alternate_screen_buffer()?;
        Ok(())
    }

    fn clear_screen(&mut self) -> Result<(), Error> {
        windows::clear_screen()
    }

    fn enable_raw_mode(&mut self) -> Result<(), Error> {
        windows::enable_raw_mode()
    }

    fn get_cursor_position(&self) -> Result<(usize, usize), Error> {
        windows::get_cursor_position()
    }

    fn get_screen_size(&self) -> Result<(usize, usize), Error> {
        windows::get_screen_size()
    }

    fn scroll_up(&self, height: usize) -> Result<(), Error> {
        windows::scroll_up_buffer(height)
    }

    fn set_cursor_position(&mut self, x: usize, y: usize) -> Result<(), Error> {
        windows::set_cursor_position(x, y)
    }

    fn set_text_attribute(&mut self, x: usize, y: usize, length: usize) -> Result<(), Error> {
        windows::set_text_attribute(x, y, length)
    }

    fn write(
        &mut self,
        x: usize,
        y: usize,
        row: &[char],
        color: Color,
        rev: bool,
    ) -> Result<(), Error> {
        windows::write_console(x, y, row, color, rev)
    }
}

// -----------------------------------------------------------------------------------------------

#[derive(Default)]
pub struct Null {
    cursor: (usize, usize),
    screen: (usize, usize),
}

impl Null {
    pub fn set_screen_size(&mut self, x: usize, y: usize) {
        self.screen = (x, y)
    }
}

#[allow(unused_variables)]
impl Terminal for Null {
    fn read_event() -> Result<Event, Error> {
        Ok(Event::from((KeyEvent::Char('a'), KeyModifier::None)))
    }

    fn alternate_screen_buffer(&mut self) -> Result<(), Error> {
        Ok(())
    }

    fn clear_screen(&mut self) -> Result<(), Error> {
        Ok(())
    }

    fn enable_raw_mode(&mut self) -> Result<(), Error> {
        Ok(())
    }

    fn get_cursor_position(&self) -> Result<(usize, usize), Error> {
        Ok(self.cursor)
    }

    fn get_screen_size(&self) -> Result<(usize, usize), Error> {
        Ok(self.screen)
    }

    fn scroll_up(&self, height: usize) -> Result<(), Error> {
        Ok(())
    }

    fn set_cursor_position(&mut self, x: usize, y: usize) -> Result<(), Error> {
        self.cursor = (x, y);
        Ok(())
    }

    fn set_text_attribute(&mut self, x: usize, y: usize, length: usize) -> Result<(), Error> {
        Ok(())
    }

    fn write(
        &mut self,
        x: usize,
        y: usize,
        row: &[char],
        color: Color,
        rev: bool,
    ) -> Result<(), Error> {
        Ok(())
    }
}
