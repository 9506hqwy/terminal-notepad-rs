use crate::console;
use crate::error::Error;
use crate::key_event::{KeyEvent, KeyModifier};
use std::sync::mpsc::channel;
use std::thread;
use std::time::Duration;

pub trait Terminal {
    fn read_key() -> Result<(KeyEvent, KeyModifier), Error>;

    fn read_key_timeout() -> Result<(KeyEvent, KeyModifier), Error> {
        let (sender, receiver) = channel();

        thread::spawn(move || sender.send(Self::read_key()).unwrap());

        loop {
            match receiver.recv_timeout(Duration::from_millis(16)) {
                Ok(ch) => return ch,
                _ => {
                    // TODO: resize screen window.
                }
            }
        }
    }

    fn alternate_screen_buffer(&mut self) -> Result<(), Error>;

    fn clear_screen(&mut self) -> Result<(), Error>;

    fn enable_raw_mode(&mut self) -> Result<(), Error>;

    fn get_cursor_position(&self) -> Result<(usize, usize), Error>;

    fn get_screen_size(&self) -> Result<(usize, usize), Error>;

    fn set_cursor_position(&mut self, x: usize, y: usize) -> Result<(), Error>;

    fn set_text_attribute(&mut self, x: usize, y: usize, length: usize) -> Result<(), Error>;

    fn write(&mut self, x: usize, y: usize, row: &[char], rev: bool) -> Result<(), Error>;
}

// -----------------------------------------------------------------------------------------------

pub struct WindowsCon;

impl Terminal for WindowsCon {
    fn read_key() -> Result<(KeyEvent, KeyModifier), Error> {
        console::read_key()
    }

    fn alternate_screen_buffer(&mut self) -> Result<(), Error> {
        console::alternate_screen_buffer()?;
        Ok(())
    }

    fn clear_screen(&mut self) -> Result<(), Error> {
        console::clear_screen()
    }

    fn enable_raw_mode(&mut self) -> Result<(), Error> {
        console::enable_raw_mode()
    }

    fn get_cursor_position(&self) -> Result<(usize, usize), Error> {
        console::get_cursor_position()
    }

    fn get_screen_size(&self) -> Result<(usize, usize), Error> {
        console::get_screen_size()
    }

    fn set_cursor_position(&mut self, x: usize, y: usize) -> Result<(), Error> {
        console::set_cursor_position(x, y)
    }

    fn set_text_attribute(&mut self, x: usize, y: usize, length: usize) -> Result<(), Error> {
        console::set_text_attribute(x, y, length)
    }

    fn write(&mut self, x: usize, y: usize, row: &[char], rev: bool) -> Result<(), Error> {
        console::write_console(x, y, row, rev)
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
    fn read_key() -> Result<(KeyEvent, KeyModifier), Error> {
        Ok((KeyEvent::Char('a'), KeyModifier::None))
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

    fn set_cursor_position(&mut self, x: usize, y: usize) -> Result<(), Error> {
        self.cursor = (x, y);
        Ok(())
    }

    fn set_text_attribute(&mut self, x: usize, y: usize, length: usize) -> Result<(), Error> {
        Ok(())
    }

    fn write(&mut self, x: usize, y: usize, row: &[char], rev: bool) -> Result<(), Error> {
        Ok(())
    }
}
