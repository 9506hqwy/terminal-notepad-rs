use crate::buffer::Buffer;
use crate::screen::Screen;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cursor {
    x0: usize,
    y0: usize,
}

impl From<(usize, usize)> for Cursor {
    fn from(value: (usize, usize)) -> Self {
        Cursor {
            x0: value.0,
            y0: value.1,
        }
    }
}

impl Cursor {
    /// Move down a row.
    pub fn move_down(&mut self, content: &Buffer) -> bool {
        let cur = self.clone();

        if self.y0 < content.rows() {
            self.y0 += 1;
            self.move_to_xmax_ifoverflow(content);
        }

        cur != *self
    }

    /// Move down a render row .
    pub fn move_down_render(&mut self, content: &Buffer) -> bool {
        let cur = self.clone();

        self.move_down(content);

        let (render, _) = cur.render(content);
        self.move_render_to_x(content, render);

        cur != *self
    }

    /// Move down a screen height.
    pub fn move_down_screen(&mut self, content: &Buffer, screen: &Screen) -> bool {
        let cur = self.clone();

        self.y0 += screen.height();
        self.move_to_ymax_ifoverflow(content);
        self.move_to_xmax_ifoverflow(content);

        cur != *self
    }

    /// Move to previous character.
    /// Move up 1 row and end of row if current is start of row.
    pub fn move_left(&mut self, content: &Buffer) -> bool {
        let cur = self.clone();

        if 0 < self.x0 {
            self.x0 -= 1;
        } else if 0 < self.y0 {
            self.y0 -= 1;
            self.x0 = content.row_char_len(self);
        }

        cur != *self
    }

    /// Move to next character.
    /// Move down 1 row and start of row if current is end of row.
    pub fn move_right(&mut self, content: &Buffer) -> bool {
        let cur = self.clone();

        if self.x0 < content.row_char_len(self) {
            self.x0 += 1;
        } else if self.y0 < content.rows() {
            self.y0 += 1;
            self.x0 = 0;
        }

        cur != *self
    }

    /// Move up a row.
    pub fn move_up(&mut self, content: &Buffer) -> bool {
        let cur = self.clone();

        if 0 < self.y0 {
            self.y0 -= 1;
            self.move_to_xmax_ifoverflow(content);
        }

        cur != *self
    }

    /// Move up a render row .
    pub fn move_up_render(&mut self, content: &Buffer) -> bool {
        let cur = self.clone();

        self.move_up(content);

        let (render, _) = cur.render(content);
        self.move_render_to_x(content, render);

        cur != *self
    }

    /// Move up a screen height.
    pub fn move_up_screen(&mut self, content: &Buffer, screen: &Screen) -> bool {
        let cur = self.clone();

        self.y0 = if screen.height() < self.y0 {
            self.y0 - screen.height()
        } else {
            0
        };

        self.move_to_xmax_ifoverflow(content);

        cur != *self
    }

    /// Move to start of row.
    pub fn move_to_x0(&mut self) -> bool {
        let cur = self.clone();

        self.x0 = 0;

        cur != *self
    }

    /// Move to end of row.
    pub fn move_to_xmax(&mut self, content: &Buffer) -> bool {
        let cur = self.clone();

        self.x0 = content.row_char_len(self);

        cur != *self
    }

    /// Returns coordinate of cursor in screen.
    pub fn render(&self, content: &Buffer) -> (usize, usize) {
        if let Some(row) = content.get(self.y0) {
            let x = row.width_range(0..self.x0);
            (x, self.y0)
        } else {
            (0, content.rows())
        }
    }

    /// Set coordinate of character axis.
    pub fn set<P: Coordinates>(&mut self, content: &Buffer, at: &P) -> bool {
        let m1 = self.set_y(content, at.y());
        let m2 = self.set_x(content, at.x());
        m1 || m2
    }

    /// Set coordinate of character X-axis.
    pub fn set_x(&mut self, content: &Buffer, x: usize) -> bool {
        let cur = self.clone();

        self.x0 = x;
        self.move_to_xmax_ifoverflow(content);

        cur != *self
    }

    /// Set coordinate of character Y-axis.
    pub fn set_y(&mut self, content: &Buffer, y: usize) -> bool {
        let cur = self.clone();

        self.y0 = y;
        self.move_to_ymax_ifoverflow(content);
        self.move_to_xmax_ifoverflow(content);

        cur != *self
    }

    fn move_render_to_x(&mut self, content: &Buffer, render: usize) -> bool {
        let cur = self.clone();

        if let Some(row) = content.get(self.y0) {
            while self.x0 < row.len() && row.width_range(0..self.x0) < render {
                self.x0 += 1;
            }

            while 0 < self.x0 && self.x0 <= row.len() && render < row.width_range(0..self.x0) {
                self.x0 -= 1;
            }
        }

        cur != *self
    }

    fn move_to_xmax_ifoverflow(&mut self, content: &Buffer) -> bool {
        let cur = self.clone();

        if content.row_char_len(self) < self.x0 {
            self.x0 = content.row_char_len(self);
        }

        cur != *self
    }

    fn move_to_ymax_ifoverflow(&mut self, content: &Buffer) -> bool {
        let cur = self.clone();

        if content.rows() < self.y0 {
            self.y0 = content.rows();
        }

        cur != *self
    }
}

// -----------------------------------------------------------------------------------------------

pub trait AsCoordinates {
    fn as_coordinates(&self) -> (usize, usize);
}

impl AsCoordinates for (usize, usize) {
    fn as_coordinates(&self) -> (usize, usize) {
        (self.0, self.1)
    }
}

impl AsCoordinates for Cursor {
    fn as_coordinates(&self) -> (usize, usize) {
        (self.x0, self.y0)
    }
}

// -----------------------------------------------------------------------------------------------

pub trait Coordinates {
    fn x(&self) -> usize;

    fn y(&self) -> usize;
}

impl Coordinates for (usize, usize) {
    fn x(&self) -> usize {
        self.0
    }

    fn y(&self) -> usize {
        self.1
    }
}

impl Coordinates for Cursor {
    fn x(&self) -> usize {
        self.x0
    }

    fn y(&self) -> usize {
        self.y0
    }
}

// -----------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal;

    #[test]
    fn move_down() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        buf.insert_row(&(0, 1), &['b']);

        let mut cur = Cursor::from((0, 0));
        let moved = cur.move_down(&buf);

        assert_eq!((0, 1), cur.as_coordinates());
        assert!(moved);
    }

    #[test]
    fn move_down_at_end() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        buf.insert_row(&(0, 1), &['b']);

        let mut cur = Cursor::from((0, 1));
        let moved = cur.move_down(&buf);

        assert_eq!((0, 2), cur.as_coordinates());
        assert!(moved);
    }

    #[test]
    fn move_down_at_yoverflow() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        buf.insert_row(&(0, 1), &['b']);

        let mut cur = Cursor::from((0, 2));
        let moved = cur.move_down(&buf);

        assert_eq!((0, 2), cur.as_coordinates());
        assert!(!moved);
    }

    #[test]
    fn move_down_render_1() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a', 'b', 'c', 'd']);
        buf.insert_row(&(0, 1), &['あ', 'い']);

        let mut cur = Cursor::from((2, 0));
        let moved = cur.move_down_render(&buf);

        assert_eq!((1, 1), cur.as_coordinates());
        assert!(moved);
    }

    #[test]
    fn move_down_render_2() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['あ', 'い']);
        buf.insert_row(&(0, 1), &['a', 'b', 'c', 'd']);

        let mut cur = Cursor::from((1, 0));
        let moved = cur.move_down_render(&buf);

        assert_eq!((2, 1), cur.as_coordinates());
        assert!(moved);
    }

    #[test]
    fn move_down_screen() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        buf.insert_row(&(0, 1), &['b']);

        let mut null = terminal::Null::default();
        null.set_screen_size(1, 3);
        let screen = Screen::current(&null).unwrap();

        let mut cur = Cursor::from((0, 0));

        let moved = cur.move_down_screen(&buf, &screen);

        assert_eq!((0, 1), cur.as_coordinates());
        assert!(moved);
    }

    #[test]
    fn move_down_screen_at_end() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        buf.insert_row(&(0, 1), &['b']);

        let mut null = terminal::Null::default();
        null.set_screen_size(1, 3);
        let screen = Screen::current(&null).unwrap();

        let mut cur = Cursor::from((0, 1));

        let moved = cur.move_down_screen(&buf, &screen);

        assert_eq!((0, 2), cur.as_coordinates());
        assert!(moved);
    }

    #[test]
    fn move_down_screen_at_yoverflow() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        buf.insert_row(&(0, 1), &['b']);

        let mut null = terminal::Null::default();
        null.set_screen_size(1, 3);
        let screen = Screen::current(&null).unwrap();

        let mut cur = Cursor::from((0, 2));

        let moved = cur.move_down_screen(&buf, &screen);

        assert_eq!((0, 2), cur.as_coordinates());
        assert!(!moved);
    }

    #[test]
    fn move_left() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a', 'b']);
        buf.insert_row(&(0, 1), &['a', 'b']);

        let mut cur = Cursor::from((2, 1));
        let moved = cur.move_left(&buf);

        assert_eq!((1, 1), cur.as_coordinates());
        assert!(moved);
    }

    #[test]
    fn move_left_at_end() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a', 'b']);
        buf.insert_row(&(0, 1), &['a', 'b']);

        let mut cur = Cursor::from((1, 1));
        let moved = cur.move_left(&buf);

        assert_eq!((0, 1), cur.as_coordinates());
        assert!(moved);
    }

    #[test]
    fn move_left_at_xunderflow() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a', 'b']);
        buf.insert_row(&(0, 1), &['a', 'b']);

        let mut cur = Cursor::from((0, 1));
        let moved = cur.move_left(&buf);

        assert_eq!((2, 0), cur.as_coordinates());
        assert!(moved);
    }

    #[test]
    fn move_right() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a', 'b']);

        let mut cur = Cursor::from((0, 0));
        let moved = cur.move_right(&buf);

        assert_eq!((1, 0), cur.as_coordinates());
        assert!(moved);
    }

    #[test]
    fn move_right_at_end() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a', 'b']);

        let mut cur = Cursor::from((1, 0));
        let moved = cur.move_right(&buf);

        assert_eq!((2, 0), cur.as_coordinates());
        assert!(moved);
    }

    #[test]
    fn move_right_at_xoverflow() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a', 'b']);

        let mut cur = Cursor::from((2, 0));
        let moved = cur.move_right(&buf);

        assert_eq!((0, 1), cur.as_coordinates());
        assert!(moved);
    }

    #[test]
    fn move_up() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        buf.insert_row(&(0, 1), &['b']);

        let mut cur = Cursor::from((0, 2));
        let moved = cur.move_up(&buf);

        assert_eq!((0, 1), cur.as_coordinates());
        assert!(moved);
    }

    #[test]
    fn move_up_at_end() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        buf.insert_row(&(0, 1), &['b']);

        let mut cur = Cursor::from((0, 1));
        let moved = cur.move_up(&buf);

        assert_eq!((0, 0), cur.as_coordinates());
        assert!(moved);
    }

    #[test]
    fn move_up_at_yunderflow() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        buf.insert_row(&(0, 1), &['b']);

        let mut cur = Cursor::from((0, 0));
        let moved = cur.move_up(&buf);

        assert_eq!((0, 0), cur.as_coordinates());
        assert!(!moved);
    }

    #[test]
    fn move_up_render_1() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a', 'b', 'c', 'd']);
        buf.insert_row(&(0, 1), &['あ', 'い']);

        let mut cur = Cursor::from((1, 1));
        let moved = cur.move_up_render(&buf);

        assert_eq!((2, 0), cur.as_coordinates());
        assert!(moved);
    }

    #[test]
    fn move_up_render_2() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['あ', 'い']);
        buf.insert_row(&(0, 1), &['a', 'b', 'c', 'd']);

        let mut cur = Cursor::from((2, 1));
        let moved = cur.move_up_render(&buf);

        assert_eq!((1, 0), cur.as_coordinates());
        assert!(moved);
    }

    #[test]
    fn move_up_screen() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        buf.insert_row(&(0, 1), &['b']);

        let mut null = terminal::Null::default();
        null.set_screen_size(1, 3);
        let screen = Screen::current(&null).unwrap();

        let mut cur = Cursor::from((0, 2));

        let moved = cur.move_up_screen(&buf, &screen);

        assert_eq!((0, 1), cur.as_coordinates());
        assert!(moved);
    }

    #[test]
    fn move_up_screen_at_end() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        buf.insert_row(&(0, 1), &['b']);

        let mut null = terminal::Null::default();
        null.set_screen_size(1, 3);
        let screen = Screen::current(&null).unwrap();

        let mut cur = Cursor::from((0, 1));

        let moved = cur.move_up_screen(&buf, &screen);

        assert_eq!((0, 0), cur.as_coordinates());
        assert!(moved);
    }

    #[test]
    fn move_up_screen_at_yunderflow() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        buf.insert_row(&(0, 1), &['b']);

        let mut null = terminal::Null::default();
        null.set_screen_size(1, 3);
        let screen = Screen::current(&null).unwrap();

        let mut cur = Cursor::from((0, 0));

        let moved = cur.move_up_screen(&buf, &screen);

        assert_eq!((0, 0), cur.as_coordinates());
        assert!(!moved);
    }

    #[test]
    fn move_to_x0() {
        let mut cur = Cursor::from((1, 0));
        let moved = cur.move_to_x0();

        assert_eq!((0, 0), cur.as_coordinates());
        assert!(moved);
    }

    #[test]
    fn move_to_x0_at_x0() {
        let mut cur = Cursor::from((0, 0));
        let moved = cur.move_to_x0();

        assert_eq!((0, 0), cur.as_coordinates());
        assert!(!moved);
    }

    #[test]
    fn move_to_xmax() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);

        let mut cur = Cursor::from((0, 0));
        let moved = cur.move_to_xmax(&buf);

        assert_eq!((1, 0), cur.as_coordinates());
        assert!(moved);
    }

    #[test]
    fn move_to_xmax_at_xmaz() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);

        let mut cur = Cursor::from((1, 0));
        let moved = cur.move_to_xmax(&buf);

        assert_eq!((1, 0), cur.as_coordinates());
        assert!(!moved);
    }

    #[test]
    fn render() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a', 'b']);
        buf.insert_row(&(0, 1), &['a', 'b']);

        let cur = Cursor::from((0, 1));
        let render = cur.render(&buf);

        assert_eq!((0, 1), render);
    }

    #[test]
    fn render_yoverflow() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a', 'b']);
        buf.insert_row(&(0, 1), &['a', 'b']);

        let cur = Cursor::from((0, 2));
        let render = cur.render(&buf);

        assert_eq!((0, 2), render);
    }

    #[test]
    fn set() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        buf.insert_row(&(0, 1), &['a', 'b']);
        buf.insert_row(&(0, 2), &['a', 'b', 'c']);

        let mut cur = Cursor::from((0, 0));
        let moved = cur.set(&buf, &(1, 1));

        assert_eq!((1, 1), cur.as_coordinates());
        assert!(moved);
    }

    #[test]
    fn set_xoverflow() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        buf.insert_row(&(0, 1), &['a', 'b']);
        buf.insert_row(&(0, 2), &['a', 'b', 'c']);

        let mut cur = Cursor::from((0, 0));
        let moved = cur.set(&buf, &(3, 1));

        assert_eq!((2, 1), cur.as_coordinates());
        assert!(moved);
    }

    #[test]
    fn set_yoverflow() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        buf.insert_row(&(0, 1), &['a', 'b']);
        buf.insert_row(&(0, 2), &['a', 'b', 'c']);

        let mut cur = Cursor::from((0, 0));
        let moved = cur.set(&buf, &(1, 4));

        assert_eq!((0, 3), cur.as_coordinates());
        assert!(moved);
    }
}
