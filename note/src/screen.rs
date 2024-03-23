use crate::buffer::{Buffer, Row};
use crate::cursor::{AsCoordinates, Coordinates};
use crate::editor::Select;
use crate::error::Error;
use crate::terminal::Terminal;
use crate::Color;
use std::cmp::{max, min};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Screen {
    left0: usize,
    top0: usize,
    height: usize,
    width: usize,
    updated: bool,
}

impl Screen {
    pub fn current(terminal: &impl Terminal) -> Result<Self, Error> {
        let mut screen = Screen::default();
        let (width, height) = terminal.get_screen_size()?;
        screen.resize(height, width);
        Ok(screen)
    }

    /// Returns the coordinates index of this screen bottom.
    pub fn bottom(&self) -> usize {
        self.top0 + (self.height - 1)
    }

    /// Clean the screen window.
    pub fn clear(&mut self, terminal: &mut impl Terminal) -> Result<(), Error> {
        terminal.scroll_up(self.height)?;
        self.updated |= true;
        Ok(())
    }

    /// Draw screen.
    pub fn draw(
        &mut self,
        content: &Buffer,
        select: &Select,
        terminal: &mut impl Terminal,
    ) -> Result<(), Error> {
        if !self.updated && !content.updated() && !select.updated() {
            return Ok(());
        }

        if self.updated {
            self.clear(terminal)?;
        }

        let end = min(content.rows(), self.bottom() + 1);
        for index in self.top0..end {
            if !self.updated && !content.row_updated(index) && !select.in_range(index) {
                continue;
            }

            let row = content.get(index).unwrap();
            let buffer = row.slice_width(self.left0..self.right() + 1);

            if !buffer.is_empty() {
                let idx = index - self.top0;

                if let Some(comment) = buffer.column().iter().position(|&ch| ch == '#') {
                    let line = buffer.column().split_at(comment);
                    terminal.write(0, idx, line.0, Color::White, false)?;
                    terminal.write(
                        buffer.width_range(0..comment),
                        idx,
                        line.1,
                        Color::Yellow,
                        false,
                    )?;
                } else {
                    terminal.write(0, idx, buffer.column(), Color::White, false)?;
                }

                if select.enabled() && select.in_range(index) {
                    if let (Some(start), Some(end)) = (select.start(), select.end()) {
                        let start = row.width_range(0..start.x());
                        let end = min(row.width_range(0..end.x()), self.right());
                        let x = if start < self.left0 {
                            0
                        } else {
                            start - self.left0
                        };
                        let width = min(end - max(start, self.left0), self.width);
                        terminal.set_text_attribute(x, index, width)?;
                    }
                }
            }
        }

        for index in end..=self.bottom() {
            let idx = index - self.top0;
            terminal.write(0, idx, &[char::from(b'~')], Color::White, false)?;
        }

        self.updated = false;
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

        self.updated |= cur != *self;
        cur != *self
    }

    /// Set need to update screen.
    pub fn force_update(&mut self) {
        self.updated |= true;
    }

    /// Returns the height of this screen.
    pub fn height(&self) -> usize {
        self.height
    }

    /// Returns the coordinates index of this screen left.
    pub fn left(&self) -> usize {
        self.left0
    }

    /// Indicates need to update screen.
    pub fn updated(&self) -> bool {
        self.updated
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

        self.updated |= cur != *self;
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

        self.updated |= cur != *self;
        cur != *self
    }

    /// Set screen size.
    pub fn resize(&mut self, height: usize, width: usize) {
        // -2 is
        // - status bar
        // - message bar
        self.height = height - 2;
        self.width = width;
        self.updated |= true;
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

#[derive(Clone)]
pub struct StatusBar {
    y0: usize,
    width: usize,
    filename: Option<String>,
    position: (usize, usize),
    updated: bool,
}

impl StatusBar {
    pub fn new(screen: &Screen, filename: Option<&str>) -> Self {
        StatusBar {
            y0: screen.height(),
            width: screen.width(),
            filename: filename.map(|f| f.to_string()),
            position: (0, 0),
            updated: true,
        }
    }

    pub fn draw(&mut self, terminal: &mut impl Terminal) -> Result<(), Error> {
        if !self.updated {
            return Ok(());
        }

        let filename = self.filename.as_deref().unwrap_or("<buffered>");
        let message = format!(
            " {:?}  {}:{}",
            filename,
            self.position.0 + 1,
            self.position.1 + 1
        );
        let mut buffer = Row::from(message);
        buffer.truncate_width(self.width);

        for _ in buffer.width()..self.width {
            buffer.append(&[char::from(b' ')]);
        }

        terminal.write(0, self.y0, buffer.column(), Color::White, true)?;

        self.updated = false;
        Ok(())
    }

    pub fn resize(&mut self, screen: &Screen) {
        self.y0 = screen.height();
        self.width = screen.width();
        self.updated |= true;
    }

    pub fn set_cursor<P: AsCoordinates>(&mut self, pos: &P) {
        let cur = self.position;
        self.position = pos.as_coordinates();
        self.updated |= cur != self.position;
    }

    pub fn set_filename(&mut self, filename: &str) {
        self.filename = Some(filename.to_string());
        self.updated |= true;
    }

    pub fn updated(&self) -> bool {
        self.updated
    }
}

// -----------------------------------------------------------------------------------------------

#[derive(Clone)]
pub struct MessageBar {
    y0: usize,
    width: usize,
    message: Row,
    updated: bool,
    fg_color: Color,
}

impl MessageBar {
    pub fn new(screen: &Screen, message: &str) -> Self {
        MessageBar {
            y0: screen.height() + 1,
            width: screen.width(),
            message: Row::from(message),
            updated: true,
            fg_color: Color::White,
        }
    }

    pub fn draw(&mut self, terminal: &mut impl Terminal) -> Result<(), Error> {
        if !self.updated {
            return Ok(());
        }

        let mut buffer = self.message.clone();
        buffer.truncate_width(self.width);
        terminal.write(0, self.y0, buffer.column(), self.fg_color, false)?;

        self.updated = false;
        Ok(())
    }

    pub fn force_update(&mut self) {
        self.updated |= true;
    }

    pub fn message(&self) -> &Row {
        &self.message
    }

    pub fn resize(&mut self, screen: &Screen) {
        self.y0 = screen.height() + 1;
        self.width = screen.width();
        self.updated |= true;
    }

    pub fn set_fg_color(&mut self, color: Color) {
        self.fg_color = color;
        self.updated |= true;
    }

    pub fn set_message(&mut self, message: Row) {
        self.message = message;
        self.updated |= true;
    }

    pub fn updated(&self) -> bool {
        self.updated
    }
}

// -----------------------------------------------------------------------------------------------

pub fn refresh_screen<T: Terminal, P: AsCoordinates + Coordinates>(
    cursor: &P,
    content: &mut Buffer,
    screen: &mut Screen,
    select: &mut Select,
    status: &mut StatusBar,
    message: &mut MessageBar,
    terminal: &mut T,
) -> Result<(), Error> {
    screen.draw(content, select, terminal)?;
    content.clear_updated();
    select.clear_updated();

    status.set_cursor(cursor);
    status.draw(terminal)?;

    message.draw(terminal)?;
    Ok(())
}

pub fn resize_screen<T: Terminal>(
    screen: &mut Screen,
    status: &mut StatusBar,
    message: &mut MessageBar,
    terminal: &mut T,
) -> Result<(), Error> {
    let (width, height) = terminal.get_screen_size()?;

    if screen.width() != width || screen.height() != height {
        screen.resize(height, width);
        status.resize(screen);
        message.resize(screen);
    }

    Ok(())
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
        assert!(screen.updated());
    }

    #[test]
    fn screen_clear() {
        let mut null = terminal::Null::default();
        null.set_screen_size(1, 3);
        let mut screen = Screen::current(&null).unwrap();
        screen.updated = false;

        screen.clear(&mut null).unwrap();

        assert!(screen.updated());
    }

    #[test]
    fn screen_draw() {
        let mut null = terminal::Null::default();
        null.set_screen_size(3, 3);
        let mut screen = Screen::current(&null).unwrap();
        screen.updated = false;

        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a', 'b', 'c', 'd', 'e']);
        buf.insert_row(&(0, 1), &['f', 'g', 'h', 'i', 'j']);

        screen.draw(&buf, &Select::default(), &mut null).unwrap();

        assert!(!screen.updated());
    }

    #[test]
    fn screen_fit_x_right() {
        let mut null = terminal::Null::default();
        null.set_screen_size(3, 3);
        let mut screen = Screen::current(&null).unwrap();
        screen.updated = false;

        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a', 'b', 'c', 'd', 'e']);
        buf.insert_row(&(0, 1), &['f', 'g', 'h', 'i', 'j']);

        let moved = screen.fit(&buf, &(4, 0));

        assert!(moved);
        assert_eq!(2, screen.left());
        assert_eq!(0, screen.top());
        assert!(screen.updated());
    }

    #[test]
    fn screen_fit_x_left() {
        let mut null = terminal::Null::default();
        null.set_screen_size(3, 3);
        let mut screen = Screen::current(&null).unwrap();
        screen.updated = false;
        screen.left0 = 2;
        screen.top0 = 0;

        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a', 'b', 'c', 'd', 'e']);
        buf.insert_row(&(0, 1), &['f', 'g', 'h', 'i', 'j']);

        let moved = screen.fit(&buf, &(1, 0));

        assert!(moved);
        assert_eq!(1, screen.left());
        assert_eq!(0, screen.top());
        assert!(screen.updated());
    }

    #[test]
    fn screen_fit_x_2_left() {
        let mut null = terminal::Null::default();
        null.set_screen_size(3, 3);
        let mut screen = Screen::current(&null).unwrap();
        screen.updated = false;
        screen.left0 = 2;
        screen.top0 = 0;

        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['あ', 'い', 'う', 'え', 'お']);

        let moved = screen.fit(&buf, &(8, 0));

        assert!(moved);
        assert_eq!(7, screen.left());
        assert_eq!(0, screen.top());
        assert!(screen.updated());
    }

    #[test]
    fn screen_fit_y_down() {
        let mut null = terminal::Null::default();
        null.set_screen_size(3, 3);
        let mut screen = Screen::current(&null).unwrap();
        screen.updated = false;

        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a', 'b', 'c', 'd', 'e']);
        buf.insert_row(&(0, 1), &['f', 'g', 'h', 'i', 'j']);

        let moved = screen.fit(&buf, &(1, 1));

        assert!(moved);
        assert_eq!(0, screen.left());
        assert_eq!(1, screen.top());
        assert!(screen.updated());
    }

    #[test]
    fn screen_fit_y_up() {
        let mut null = terminal::Null::default();
        null.set_screen_size(3, 3);
        let mut screen = Screen::current(&null).unwrap();
        screen.updated = false;
        screen.left0 = 1;
        screen.top0 = 1;

        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a', 'b', 'c', 'd', 'e']);
        buf.insert_row(&(0, 1), &['f', 'g', 'h', 'i', 'j']);

        let moved = screen.fit(&buf, &(2, 0));

        assert!(moved);
        assert_eq!(1, screen.left());
        assert_eq!(0, screen.top());
        assert!(screen.updated());
    }

    #[test]
    fn screen_fit_notmoved() {
        let mut null = terminal::Null::default();
        null.set_screen_size(3, 3);
        let mut screen = Screen::current(&null).unwrap();
        screen.updated = false;

        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a', 'b', 'c', 'd', 'e']);
        buf.insert_row(&(0, 1), &['f', 'g', 'h', 'i', 'j']);

        let moved = screen.fit(&buf, &(2, 0));

        assert!(!moved);
        assert_eq!(0, screen.left());
        assert_eq!(0, screen.top());
        assert!(!screen.updated());
    }

    #[test]
    fn screen_move_down() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        buf.insert_row(&(0, 1), &['b']);

        let mut null = terminal::Null::default();
        null.set_screen_size(1, 3);
        let mut screen = Screen::current(&null).unwrap();
        screen.updated = false;

        let moved = screen.move_down(&buf);

        assert!(moved);
        assert_eq!(0, screen.left());
        assert_eq!(1, screen.top());
        assert!(screen.updated());
    }

    #[test]
    fn screen_move_down_yoverflow() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        buf.insert_row(&(0, 1), &['b']);

        let mut null = terminal::Null::default();
        null.set_screen_size(1, 3);
        let mut screen = Screen::current(&null).unwrap();
        screen.updated = false;
        screen.top0 = 2;

        let moved = screen.move_down(&buf);

        assert!(!moved);
        assert_eq!(0, screen.left());
        assert_eq!(2, screen.top());
        assert!(!screen.updated());
    }

    #[test]
    fn screen_move_up() {
        let mut null = terminal::Null::default();
        null.set_screen_size(1, 3);
        let mut screen = Screen::current(&null).unwrap();
        screen.updated = false;
        screen.top0 = 1;

        let moved = screen.move_up();

        assert!(moved);
        assert_eq!(0, screen.left());
        assert_eq!(0, screen.top());
        assert!(screen.updated());
    }

    #[test]
    fn screen_move_up_yunderflow() {
        let mut null = terminal::Null::default();
        null.set_screen_size(1, 3);
        let mut screen = Screen::current(&null).unwrap();
        screen.updated = false;

        let moved = screen.move_up();

        assert!(!moved);
        assert_eq!(0, screen.left());
        assert_eq!(0, screen.top());
        assert!(!screen.updated());
    }

    // -------------------------------------------------------------------------------------------

    #[test]
    fn status_bar_draw() {
        let mut null = terminal::Null::default();
        null.set_screen_size(3, 3);
        let screen = Screen::current(&null).unwrap();

        let mut bar = StatusBar::new(&screen, None);

        bar.set_cursor(&(0, 0));
        bar.draw(&mut null).unwrap();
    }

    // -------------------------------------------------------------------------------------------

    #[test]
    fn message_bar_draw() {
        let mut null = terminal::Null::default();
        null.set_screen_size(3, 3);
        let screen = Screen::current(&null).unwrap();

        let mut bar = MessageBar::new(&screen, "");

        bar.draw(&mut null).unwrap();
    }
}
