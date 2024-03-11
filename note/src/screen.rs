use crate::buffer::{Buffer, Row};
use crate::cursor::Coordinates;
use crate::error::Error;
use crate::terminal::Terminal;
use std::cmp::min;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Screen {
    left0: usize,
    top0: usize,
    height: usize,
    width: usize,
}

impl Screen {
    pub fn current(terminal: &impl Terminal) -> Result<Self, Error> {
        let (width, height) = terminal.get_screen_size()?;
        Ok(Screen {
            width,
            // -2 is
            // - status bar
            // - message bar
            height: height - 2,
            ..Default::default()
        })
    }

    /// Returns the coordinates index of this screen bottom.
    pub fn bottom(&self) -> usize {
        self.top0 + (self.height - 1)
    }

    /// Clean the screen window.
    pub fn clear(&mut self, terminal: &mut impl Terminal) -> Result<(), Error> {
        terminal.clear_screen()?;
        Ok(())
    }

    /// Draw screen.
    pub fn draw(&self, content: &Buffer, terminal: &mut impl Terminal) -> Result<(), Error> {
        let end = min(content.rows(), self.bottom() + 1);
        for index in self.top0..end {
            let row = content.get(index).unwrap();
            let buffer = row.slice_width(self.left0..self.right() + 1);

            if !buffer.is_empty() {
                let idx = index - self.top0;
                terminal.write(0, idx, buffer.column(), false)?;
            }
        }

        for index in end..=self.bottom() {
            let idx = index - self.top0;
            terminal.write(0, idx, &[char::from(b'~')], false)?;
        }

        Ok(())
    }

    /// Move the screen window if the position is out of the window.
    pub fn fit<P: Coordinates>(&mut self, content: &Buffer, pos: &P) -> bool {
        let cur = self.clone();

        match pos.y() {
            y if y < self.top0 => self.top0 = y,
            y if self.bottom() < y => self.top0 = y - (self.height - 1),
            _ => {}
        }

        match pos.x() {
            x if x < self.left0 => self.left0 = x,
            x if self.right() <= x => {
                // include `=` bacause considering  that last char is multi width.
                if let Some(row) = content.get(pos.y()) {
                    self.left0 = x - (self.width - row.last_char_width());
                } else {
                    self.left0 = 0;
                }
            }
            _ => {}
        }

        cur != *self
    }

    /// Returns the height of this screen.
    pub fn height(&self) -> usize {
        self.height
    }

    /// Returns the coordinates index of this screen left.
    pub fn left(&self) -> usize {
        self.left0
    }

    /// Move down a height.
    pub fn move_down(&mut self, content: &Buffer) -> bool {
        let cur = self.clone();

        if self.height < content.rows() {
            self.top0 += self.height;
            if content.rows() < self.bottom() {
                self.top0 = content.rows() - (self.height - 1);
            }
        }

        cur != *self
    }

    /// Move up a height.
    pub fn move_up(&mut self) -> bool {
        let cur = self.clone();

        self.top0 = if self.height < self.top0 {
            self.top0 - self.height
        } else {
            0
        };

        cur != *self
    }

    /// Returns the coordinates index of this screen right.
    pub fn right(&self) -> usize {
        self.left0 + (self.width - 1)
    }

    /// Returns the coordinates index of this screen top.
    pub fn top(&self) -> usize {
        self.top0
    }

    /// Returns the width of this screen.
    pub fn width(&self) -> usize {
        self.width
    }
}

// -----------------------------------------------------------------------------------------------

pub struct StatusBar {
    y0: usize,
    width: usize,
    filename: Option<String>,
}

impl StatusBar {
    pub fn new(screen: &Screen, filename: Option<&str>) -> Self {
        StatusBar {
            y0: screen.bottom() + 1,
            width: screen.width(),
            filename: filename.map(|f| f.to_string()),
        }
    }

    pub fn draw<P: Coordinates>(&self, pos: &P, terminal: &mut impl Terminal) -> Result<(), Error> {
        let filename = self.filename.as_deref().unwrap_or("<buffered>");
        let message = format!(" {:?}  {}:{}", filename, pos.y() + 1, pos.x() + 1);
        let mut buffer = Row::from(message.chars().collect::<Vec<char>>());
        buffer.truncate_width(self.width);

        for _ in buffer.width()..self.width {
            buffer.append(&[char::from(b' ')]);
        }

        terminal.write(0, self.y0, buffer.column(), true)?;

        Ok(())
    }

    pub fn set_filename(&mut self, filename: &str) {
        self.filename = Some(filename.to_string());
    }
}

// -----------------------------------------------------------------------------------------------

pub struct MessageBar {
    y0: usize,
    width: usize,
    message: Row,
}

impl MessageBar {
    pub fn new(screen: &Screen, message: &str) -> Self {
        MessageBar {
            y0: screen.bottom() + 2,
            width: screen.width(),
            message: Row::from(message.chars().collect::<Vec<char>>()),
        }
    }

    pub fn draw(&self, terminal: &mut impl Terminal) -> Result<(), Error> {
        let mut buffer = self.message.clone();
        buffer.truncate_width(self.width);
        terminal.write(0, self.y0, buffer.column(), false)?;

        Ok(())
    }
}

// -----------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal;

    #[test]
    fn screen_current() {
        let mut null = terminal::Null::default();
        null.set_screen_size(20, 10);

        let screen = Screen::current(&null).unwrap();

        assert_eq!(0, screen.left());
        assert_eq!(0, screen.top());
        assert_eq!(19, screen.right());
        assert_eq!(7, screen.bottom());
        assert_eq!(8, screen.height());
        assert_eq!(20, screen.width());
    }

    #[test]
    fn screen_clear() {
        let mut null = terminal::Null::default();
        null.set_screen_size(1, 3);
        let mut screen = Screen::current(&null).unwrap();

        screen.clear(&mut null).unwrap();
    }

    #[test]
    fn screen_draw() {
        let mut null = terminal::Null::default();
        null.set_screen_size(3, 3);
        let screen = Screen::current(&null).unwrap();

        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a', 'b', 'c', 'd', 'e']);
        buf.insert_row(&(0, 1), &['f', 'g', 'h', 'i', 'j']);

        screen.draw(&buf, &mut null).unwrap();
    }

    #[test]
    fn screen_fit_x_right() {
        let mut null = terminal::Null::default();
        null.set_screen_size(3, 3);
        let mut screen = Screen::current(&null).unwrap();

        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a', 'b', 'c', 'd', 'e']);
        buf.insert_row(&(0, 1), &['f', 'g', 'h', 'i', 'j']);

        let moved = screen.fit(&buf, &(4, 0));

        assert!(moved);
        assert_eq!(2, screen.left());
        assert_eq!(0, screen.top());
    }

    #[test]
    fn screen_fit_x_left() {
        let mut null = terminal::Null::default();
        null.set_screen_size(3, 3);
        let mut screen = Screen::current(&null).unwrap();
        screen.left0 = 2;
        screen.top0 = 0;

        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a', 'b', 'c', 'd', 'e']);
        buf.insert_row(&(0, 1), &['f', 'g', 'h', 'i', 'j']);

        let moved = screen.fit(&buf, &(1, 0));

        assert!(moved);
        assert_eq!(1, screen.left());
        assert_eq!(0, screen.top());
    }

    #[test]
    fn screen_fit_x_2_left() {
        let mut null = terminal::Null::default();
        null.set_screen_size(3, 3);
        let mut screen = Screen::current(&null).unwrap();
        screen.left0 = 2;
        screen.top0 = 0;

        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['あ', 'い', 'う', 'え', 'お']);

        let moved = screen.fit(&buf, &(8, 0));

        assert!(moved);
        assert_eq!(7, screen.left());
        assert_eq!(0, screen.top());
    }

    #[test]
    fn screen_fit_y_down() {
        let mut null = terminal::Null::default();
        null.set_screen_size(3, 3);
        let mut screen = Screen::current(&null).unwrap();

        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a', 'b', 'c', 'd', 'e']);
        buf.insert_row(&(0, 1), &['f', 'g', 'h', 'i', 'j']);

        let moved = screen.fit(&buf, &(1, 1));

        assert!(moved);
        assert_eq!(0, screen.left());
        assert_eq!(1, screen.top());
    }

    #[test]
    fn screen_fit_y_up() {
        let mut null = terminal::Null::default();
        null.set_screen_size(3, 3);
        let mut screen = Screen::current(&null).unwrap();
        screen.left0 = 1;
        screen.top0 = 1;

        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a', 'b', 'c', 'd', 'e']);
        buf.insert_row(&(0, 1), &['f', 'g', 'h', 'i', 'j']);

        let moved = screen.fit(&buf, &(2, 0));

        assert!(moved);
        assert_eq!(1, screen.left());
        assert_eq!(0, screen.top());
    }

    #[test]
    fn screen_fit_notmoved() {
        let mut null = terminal::Null::default();
        null.set_screen_size(3, 3);
        let mut screen = Screen::current(&null).unwrap();

        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a', 'b', 'c', 'd', 'e']);
        buf.insert_row(&(0, 1), &['f', 'g', 'h', 'i', 'j']);

        let moved = screen.fit(&buf, &(2, 0));

        assert!(!moved);
        assert_eq!(0, screen.left());
        assert_eq!(0, screen.top());
    }

    #[test]
    fn screen_move_down() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        buf.insert_row(&(0, 1), &['b']);

        let mut null = terminal::Null::default();
        null.set_screen_size(1, 3);
        let mut screen = Screen::current(&null).unwrap();

        let moved = screen.move_down(&buf);

        assert_eq!(0, screen.left());
        assert_eq!(1, screen.top());
        assert!(moved);
    }

    #[test]
    fn screen_move_down_yoverflow() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        buf.insert_row(&(0, 1), &['b']);

        let mut null = terminal::Null::default();
        null.set_screen_size(1, 3);
        let mut screen = Screen::current(&null).unwrap();
        screen.top0 = 2;

        let moved = screen.move_down(&buf);

        assert_eq!(0, screen.left());
        assert_eq!(2, screen.top());
        assert!(!moved);
    }

    #[test]
    fn screen_move_up() {
        let mut null = terminal::Null::default();
        null.set_screen_size(1, 3);
        let mut screen = Screen::current(&null).unwrap();
        screen.top0 = 1;

        let moved = screen.move_up();

        assert_eq!(0, screen.left());
        assert_eq!(0, screen.top());
        assert!(moved);
    }

    #[test]
    fn screen_move_up_yunderflow() {
        let mut null = terminal::Null::default();
        null.set_screen_size(1, 3);
        let mut screen = Screen::current(&null).unwrap();

        let moved = screen.move_up();

        assert_eq!(0, screen.left());
        assert_eq!(0, screen.top());
        assert!(!moved);
    }

    // -------------------------------------------------------------------------------------------

    #[test]
    fn status_bar_draw() {
        let mut null = terminal::Null::default();
        null.set_screen_size(3, 3);
        let screen = Screen::current(&null).unwrap();

        let bar = StatusBar::new(&screen, None);

        bar.draw(&(0, 0), &mut null).unwrap();
    }

    // -------------------------------------------------------------------------------------------

    #[test]
    fn message_bar_draw() {
        let mut null = terminal::Null::default();
        null.set_screen_size(3, 3);
        let screen = Screen::current(&null).unwrap();

        let bar = MessageBar::new(&screen, "");

        bar.draw(&mut null).unwrap();
    }
}
