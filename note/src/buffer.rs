use crate::cursor::{AsCoordinates, Coordinates, Cursor};
use crate::error::Error;
use crate::history::{History, Operation};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::iter;
use std::ops::Range;
use std::path::{Path, PathBuf};
use unicode_width::UnicodeWidthChar;

const TAB_STOP: usize = 8;

#[derive(Default)]
pub struct Buffer {
    rows: Vec<Row>,
    filename: Option<PathBuf>,
    cached: bool,
    updated: Vec<Range<usize>>,
    history: History<(usize, usize)>,
    pending: Option<Row>,
}

impl TryFrom<Option<&Path>> for Buffer {
    type Error = Error;

    fn try_from(value: Option<&Path>) -> Result<Self, Self::Error> {
        let mut buffer = Buffer::default();

        if let Some(path) = value {
            let file = File::open(path)?;
            let reader = BufReader::new(file);
            for line in reader.lines() {
                let chars = Row::from(line?.chars().collect::<Vec<char>>());
                buffer.rows.push(chars);
            }
        }

        buffer.filename = value.map(PathBuf::from);

        Ok(buffer)
    }
}

impl Buffer {
    pub fn append_row<P: Coordinates + AsCoordinates>(&mut self, at: &P, text: &[char]) {
        if let Some(cur) = self.append_row_bypass(at, text) {
            self.history
                .record(at.as_coordinates(), Operation::Append(cur));
        }
    }

    pub fn append_row_bypass<P: Coordinates + AsCoordinates>(
        &mut self,
        at: &P,
        text: &[char],
    ) -> Option<(usize, usize)> {
        if let Some(row) = self.rows.get_mut(at.y()) {
            self.cached = true;
            self.updated.push(at.y()..at.y() + 1);
            let x = row.len();
            row.append(text);
            Some((x, at.y()))
        } else {
            None
        }
    }

    pub fn cached(&self) -> bool {
        self.cached
    }

    pub fn clear_updated(&mut self) {
        self.updated.clear();
    }

    pub fn delete_row<P: Coordinates + AsCoordinates>(&mut self, at: &P) -> Option<Row> {
        let row = self.delete_row_bypass(at);
        if let Some(r) = row.as_ref() {
            self.history.record(
                at.as_coordinates(),
                Operation::DeleteRow(at.as_coordinates(), r.clone()),
            );
        }
        row
    }

    pub fn delete_row_bypass<P: Coordinates + AsCoordinates>(&mut self, at: &P) -> Option<Row> {
        if at.y() < self.rows() {
            self.cached = true;
            self.updated.push(at.y()..self.rows());
            Some(self.rows.remove(at.y()))
        } else {
            None
        }
    }

    pub fn delete_char<P: Coordinates + AsCoordinates>(&mut self, at: &P) {
        if let Some(ch) = self.delete_char_bypass(at) {
            self.history.record(
                at.as_coordinates(),
                Operation::DeleteChar(at.as_coordinates(), ch),
            );
        }
    }

    pub fn delete_char_bypass<P: Coordinates + AsCoordinates>(&mut self, at: &P) -> Option<char> {
        if let Some(row) = self.rows.get_mut(at.y()) {
            if 0 < at.x() && at.x() <= row.len() {
                if let Some(ch) = row.remove(at.x() - 1) {
                    self.cached = true;
                    self.updated.push(at.y()..at.y() + 1);
                    return Some(ch);
                }
            }
        }

        None
    }

    pub fn delete_chars<P: Coordinates + AsCoordinates>(&mut self, at: &P, length: usize) {
        if let Some(text) = self.delete_chars_bypass(at, length) {
            self.history.record(
                at.as_coordinates(),
                Operation::DeleteChars(at.as_coordinates(), Row::from(text)),
            );
        }
    }

    pub fn delete_chars_bypass<P: Coordinates + AsCoordinates>(
        &mut self,
        at: &P,
        length: usize,
    ) -> Option<Vec<char>> {
        if let Some(row) = self.rows.get_mut(at.y()) {
            if at.x() < row.len() {
                if let Some(text) = row.remove_range(at.x()..at.x() + length) {
                    self.cached = true;
                    self.updated.push(at.y()..at.y() + 1);
                    return Some(text);
                }
            }
        }

        None
    }

    pub fn find_at<P: Coordinates>(&self, at: &P, keyword: &str) -> Option<(usize, usize)> {
        let mut skip_x = at.x();
        for (y, c) in self.rows.iter().enumerate().skip(at.y()) {
            let row = c.to_string_at(skip_x);
            if let Some(x) = row.find(keyword) {
                return Some((x + skip_x, y));
            }

            skip_x = 0;
        }

        None
    }

    pub fn get(&self, index: usize) -> Option<&Row> {
        self.rows.get(index)
    }

    pub fn insert_row<P: Coordinates + AsCoordinates>(&mut self, at: &P, text: &[char]) {
        self.insert_row_bypass(at, text);
        self.history.record(
            at.as_coordinates(),
            Operation::InsertRow(at.as_coordinates()),
        );
    }

    pub fn insert_row_bypass<P: Coordinates + AsCoordinates>(&mut self, at: &P, text: &[char]) {
        self.cached = true;
        self.updated.push(at.y()..self.rows() + 1);
        self.rows.insert(at.y(), Row::from(text));
    }

    pub fn insert_char<P: Coordinates + AsCoordinates>(&mut self, at: &P, ch: char) {
        if self.insert_char_bypass(at, ch).is_some() {
            self.history.record(
                at.as_coordinates(),
                Operation::InsertChar(at.as_coordinates()),
            );
        }
    }

    pub fn insert_char_bypass<P: Coordinates + AsCoordinates>(
        &mut self,
        at: &P,
        ch: char,
    ) -> Option<(usize, usize)> {
        if let Some(row) = self.rows.get_mut(at.y()) {
            if at.x() <= row.len() {
                self.cached = true;
                self.updated.push(at.y()..at.y() + 1);
                row.insert(at.x(), ch);
                return Some((at.x(), at.y()));
            }
        }

        None
    }

    pub fn insert_chars<P: Coordinates + AsCoordinates>(&mut self, at: &P, text: &[char]) {
        if self.insert_chars_bypass(at, text).is_some() {
            self.history.record(
                at.as_coordinates(),
                Operation::InsertChars(at.as_coordinates(), text.len()),
            );
        }
    }

    pub fn insert_chars_bypass<P: Coordinates + AsCoordinates>(
        &mut self,
        at: &P,
        text: &[char],
    ) -> Option<(usize, usize)> {
        if let Some(row) = self.rows.get_mut(at.y()) {
            if at.x() <= row.len() {
                self.cached = true;
                self.updated.push(at.y()..at.y() + 1);
                row.insert_slice(at.x(), text);
                return Some((at.x(), at.y()));
            }
        }

        None
    }

    pub fn rfind_at<P: Coordinates>(&self, at: &P, keyword: &str) -> Option<(usize, usize)> {
        let rkeyword = keyword.chars().rev().collect::<String>();
        let mut skip_x = if at.y() < self.rows() {
            at.x()
        } else {
            usize::MAX
        };
        for (y, c) in self.rows.iter().enumerate().take(at.y() + 1).rev() {
            let taken = if skip_x == usize::MAX {
                c.len()
            } else {
                skip_x + 1
            };
            let row = c.rev_at(taken).to_string_at(0);

            if let Some(x) = row.find(&rkeyword) {
                return Some((row.len() - x - keyword.len(), y));
            }

            skip_x = usize::MAX;
        }

        None
    }

    pub fn row_char_len<P: Coordinates>(&self, at: &P) -> usize {
        self.rows.get(at.y()).map(|r| r.len()).unwrap_or_default()
    }

    pub fn row_updated(&self, row: usize) -> bool {
        self.updated.iter().any(|r| r.start <= row && row < r.end)
    }

    pub fn rows(&self) -> usize {
        self.rows.len()
    }

    pub fn paste_pending<P: Coordinates + AsCoordinates>(&mut self, at: &P) {
        if let Some(row) = self.pending.clone() {
            self.insert_chars(at, row.column());
        }
    }

    pub fn pending(&self) -> Option<&Row> {
        self.pending.as_ref()
    }

    pub fn save(&mut self) -> Result<(), Error> {
        if let Some(path) = self.filename.clone() {
            self.save_as(&path)?;
        }

        Ok(())
    }

    pub fn save_as(&mut self, path: &Path) -> Result<(), Error> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        for row in &self.rows {
            let buf = row.to_string_at(0);
            writer.write_all(buf.as_bytes())?;
            writer.write_all("\r\n".as_bytes())?;
        }

        writer.flush()?;

        self.cached = false;

        Ok(())
    }

    pub fn set_filename(&mut self, filename: &Path) {
        self.filename = Some(PathBuf::from(filename));
    }

    pub fn shrink_row<P: Coordinates + AsCoordinates>(&mut self, at: &P) {
        if let Some(row) = self.shrink_row_bypass(at) {
            self.history.record(
                at.as_coordinates(),
                Operation::ShrinkRow(at.as_coordinates(), row),
            );
        }
    }

    pub fn shrink_row_bypass<P: Coordinates + AsCoordinates>(&mut self, at: &P) -> Option<Row> {
        if let Some(row) = self.rows.get_mut(at.y()) {
            self.cached = true;
            self.updated.push(at.y()..at.y() + 1);
            self.pending = Some(row.split_off(at.x()));
            self.pending.clone()
        } else {
            None
        }
    }

    pub fn split_row<P: Coordinates + AsCoordinates>(&mut self, at: &P) {
        if let Some(cur) = self.split_row_bypass(at) {
            self.history
                .record(at.as_coordinates(), Operation::SplitRow(cur));
        }
    }

    pub fn split_row_bypass<P: Coordinates + AsCoordinates>(
        &mut self,
        at: &P,
    ) -> Option<(usize, usize)> {
        let row_len = self.rows();
        if let Some(row) = self.rows.get_mut(at.y()) {
            self.cached = true;
            self.updated.push(at.y()..row_len + 1);

            let next = row.split_off(at.x());

            let mut next_at = Cursor::default();
            next_at.set(self, &(at.x(), at.y() + 1));

            self.insert_row_bypass(&next_at, next.column());

            Some(next_at.as_coordinates())
        } else {
            None
        }
    }

    pub fn squash_row<P: Coordinates + AsCoordinates>(&mut self, at: &P) {
        if let Some(cur) = self.squash_row_bypass(at) {
            self.history
                .record(at.as_coordinates(), Operation::SquashRow(cur));
        }
    }

    pub fn squash_row_bypass<P: Coordinates + AsCoordinates>(
        &mut self,
        at: &P,
    ) -> Option<(usize, usize)> {
        if 0 < at.y() {
            if let Some(row) = self.delete_row_bypass(at) {
                self.cached = true;
                self.updated.push(at.y() - 1..self.rows());

                let mut next_at = Cursor::default();
                next_at.set(self, &(at.x(), at.y() - 1));

                return self.append_row_bypass(&next_at, row.column());
            }
        }

        None
    }

    pub fn undo(&mut self) -> Option<(usize, usize)> {
        if let Some(history) = self.history.rollback() {
            self.cached = true;
            let cord = match history {
                (cur, Operation::Append(cord)) => {
                    self.shrink_row_bypass(&cord);
                    cur
                }
                (cur, Operation::DeleteChar(cord, ch)) => {
                    self.insert_char_bypass(&(cord.0 - 1, cord.1), ch);
                    cur
                }
                (cur, Operation::DeleteChars(cord, row)) => {
                    self.insert_chars_bypass(&cord, row.column());
                    cur
                }
                (cur, Operation::DeleteRow(cord, row)) => {
                    self.insert_row_bypass(&cord, row.column());
                    cur
                }
                (cur, Operation::InsertChar(cord)) => {
                    self.delete_char_bypass(&(cord.0 + 1, cord.1));
                    cur
                }
                (cur, Operation::InsertChars(cord, length)) => {
                    self.delete_chars_bypass(&cord, length);
                    cur
                }
                (cur, Operation::InsertRow(cord)) => {
                    self.delete_row_bypass(&cord);
                    cur
                }
                (cur, Operation::ShrinkRow(cord, row)) => {
                    self.append_row_bypass(&cord, row.column());
                    cur
                }
                (cur, Operation::SplitRow(cord)) => {
                    self.squash_row_bypass(&cord);
                    cur
                }
                (cur, Operation::SquashRow(cord)) => {
                    self.split_row_bypass(&cord);
                    cur
                }
            };
            Some(cord)
        } else {
            None
        }
    }

    pub fn updated(&self) -> bool {
        !self.updated.is_empty()
    }
}

// -----------------------------------------------------------------------------------------------

#[derive(Clone, Default)]
pub struct Row {
    column: Vec<char>,
}

impl From<Vec<char>> for Row {
    fn from(value: Vec<char>) -> Self {
        Row { column: value }
    }
}

impl From<&[char]> for Row {
    fn from(value: &[char]) -> Self {
        Row {
            column: value.to_vec(),
        }
    }
}

impl Row {
    pub fn append(&mut self, other: &[char]) {
        self.column.extend_from_slice(other)
    }

    pub fn column(&self) -> &[char] {
        &self.column
    }

    pub fn insert(&mut self, index: usize, element: char) {
        if index <= self.column.len() {
            self.column.insert(index, element);
        }
    }

    pub fn insert_slice(&mut self, index: usize, other: &[char]) {
        if index <= self.column.len() {
            let removed = self.column.split_off(index);
            self.column.extend_from_slice(other);
            self.column.extend(removed);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.column.is_empty()
    }

    pub fn last_char_width(&self) -> usize {
        match self.column.last() {
            Some(&ch) => char_width(ch),
            _ => 0,
        }
    }

    pub fn len(&self) -> usize {
        self.column.len()
    }

    pub fn shrink_width(&mut self, min_width: usize) -> usize {
        if self.width() <= min_width {
            let width = self.width();
            self.column.clear();
            return width;
        }

        for index in 0..self.column.len() {
            let width = self.width_range(0..index);
            if min_width <= width {
                self.column.drain(..index);
                return width;
            }
        }

        0
    }

    pub fn slice_width(&self, range: Range<usize>) -> Row {
        let mut render = self.render();

        let removed = render.shrink_width(range.start);
        if range.start < removed {
            for _ in 0..(removed - range.start) {
                render.insert(0, ' ')
            }
        }

        let width = render.truncate_width(range.end - range.start);
        for _ in width..(range.end - range.start) {
            render.append(&[' '])
        }

        render
    }

    pub fn split_off(&mut self, at: usize) -> Row {
        Row::from(self.column.split_off(at))
    }

    pub fn to_string_at(&self, at: usize) -> String {
        self.column.iter().skip(at).collect()
    }

    pub fn truncate_width(&mut self, max_width: usize) -> usize {
        for index in 0..self.column.len() {
            if max_width < self.width_range(0..index + 1) {
                self.column.truncate(index);
                break;
            }
        }

        self.width()
    }

    pub fn remove(&mut self, index: usize) -> Option<char> {
        if index < self.column.len() {
            Some(self.column.remove(index))
        } else {
            None
        }
    }

    pub fn remove_range(&mut self, range: Range<usize>) -> Option<Vec<char>> {
        if range.end <= self.column.len() {
            Some(self.column.drain(range).collect())
        } else {
            None
        }
    }

    pub fn rev_at(&self, at: usize) -> Row {
        let rev = self
            .column
            .iter()
            .take(at)
            .rev()
            .cloned()
            .collect::<Vec<char>>();
        Row::from(rev)
    }

    pub fn width(&self) -> usize {
        self.width_range(0..self.column.len())
    }

    pub fn width_range(&self, range: Range<usize>) -> usize {
        let mut render = 0;

        for &ch in &self.column[range] {
            if ch == '\t' {
                render += TAB_STOP - (render % TAB_STOP);
            } else {
                render += char_width(ch);
            }
        }

        render
    }

    fn render(&self) -> Row {
        let mut render = Row::default();

        for &ch in &self.column {
            if ch == '\t' {
                let next_tab_stop = TAB_STOP - (render.width() % TAB_STOP);
                let spaces = iter::repeat(char::from(b' '))
                    .take(next_tab_stop)
                    .collect::<Vec<char>>();
                render.column.extend_from_slice(&spaces);
            } else {
                render.column.push(ch)
            }
        }

        render
    }
}

// -----------------------------------------------------------------------------------------------

fn char_width(ch: char) -> usize {
    ch.width_cjk().unwrap_or(1)
}

// -----------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn init_screen(buf: &mut Buffer) {
        buf.cached = false;
        buf.clear_updated();
        buf.history.clear();
    }

    #[test]
    fn buffer_append_row() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        init_screen(&mut buf);

        buf.append_row(&(0, 0), &['b']);

        assert_eq!(&['a', 'b'], buf.rows[0].column());
        assert!(buf.cached());
        assert!(buf.updated());
        assert_eq!(1, buf.history.len());
    }

    #[test]
    fn buffer_append_row_yoverflow() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        init_screen(&mut buf);

        buf.append_row(&(0, 1), &['b']);

        assert_eq!(&['a'], buf.rows[0].column());
        assert!(!buf.cached());
        assert!(!buf.updated());
        assert_eq!(0, buf.history.len());
    }

    #[test]
    fn buffer_delete_row() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        init_screen(&mut buf);

        buf.delete_row(&(0, 0));

        assert_eq!(0, buf.rows());
        assert!(buf.cached());
        assert!(buf.updated());
        assert_eq!(1, buf.history.len());
    }

    #[test]
    fn buffer_delete_row_yoverflow() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        init_screen(&mut buf);

        buf.delete_row(&(0, 1));

        assert_eq!(1, buf.rows());
        assert!(!buf.cached());
        assert!(!buf.updated());
        assert_eq!(0, buf.history.len());
    }

    #[test]
    fn buffer_delete_char() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a', 'b']);
        init_screen(&mut buf);

        buf.delete_char(&(1, 0));

        assert_eq!(&['b'], buf.rows[0].column());
        assert!(buf.cached());
        assert!(buf.updated());
        assert_eq!(1, buf.history.len());
    }

    #[test]
    fn buffer_delete_char_xoverflow() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a', 'b']);
        init_screen(&mut buf);

        buf.delete_char(&(3, 0));

        assert_eq!(&['a', 'b'], buf.rows[0].column());
        assert!(!buf.cached());
        assert!(!buf.updated());
        assert_eq!(0, buf.history.len());
    }

    #[test]
    fn buffer_delete_char_yoverflow() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a', 'b']);
        init_screen(&mut buf);

        buf.delete_char(&(1, 1));

        assert_eq!(&['a', 'b'], buf.rows[0].column());
        assert!(!buf.cached());
        assert!(!buf.updated());
        assert_eq!(0, buf.history.len());
    }

    #[test]
    fn buffer_delete_chars() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a', 'b']);
        init_screen(&mut buf);

        buf.delete_chars(&(1, 0), 1);

        assert_eq!(&['a'], buf.rows[0].column());
        assert!(buf.cached());
        assert!(buf.updated());
        assert_eq!(1, buf.history.len());
    }

    #[test]
    fn buffer_delete_chars_xoverflow() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a', 'b']);
        init_screen(&mut buf);

        buf.delete_chars(&(3, 0), 1);

        assert_eq!(&['a', 'b'], buf.rows[0].column());
        assert!(!buf.cached());
        assert!(!buf.updated());
        assert_eq!(0, buf.history.len());
    }

    #[test]
    fn buffer_delete_chars_yoverflow() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a', 'b']);
        init_screen(&mut buf);

        buf.delete_chars(&(1, 1), 1);

        assert_eq!(&['a', 'b'], buf.rows[0].column());
        assert!(!buf.cached());
        assert!(!buf.updated());
        assert_eq!(0, buf.history.len());
    }

    #[test]
    fn buffer_find_at_0() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a', 'b', 'c']);
        buf.insert_row(&(0, 1), &['a', 'b', 'c']);
        buf.insert_row(&(0, 2), &['a', 'b', 'c']);
        init_screen(&mut buf);

        let at = buf.find_at(&(0, 0), "bc");

        assert_eq!(Some((1, 0)), at);
    }

    #[test]
    fn buffer_find_at_1() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a', 'b', 'c']);
        buf.insert_row(&(0, 1), &['a', 'b', 'c']);
        buf.insert_row(&(0, 2), &['a', 'b', 'c']);
        init_screen(&mut buf);

        let at = buf.find_at(&(2, 1), "bc");

        assert_eq!(Some((1, 2)), at);
    }

    #[test]
    fn buffer_find_at_notfound() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a', 'b', 'c']);
        buf.insert_row(&(0, 1), &['a', 'b', 'c']);
        buf.insert_row(&(0, 2), &['a', 'b', 'c']);
        init_screen(&mut buf);

        let at = buf.find_at(&(2, 2), "bc");

        assert_eq!(None, at);
    }

    #[test]
    fn buffer_get() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        init_screen(&mut buf);

        let row = buf.get(0);

        assert!(row.is_some());
    }

    #[test]
    fn buffer_get_notfound() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        init_screen(&mut buf);

        let row = buf.get(1);

        assert!(row.is_none());
    }

    #[test]
    fn buffer_insert_row_0() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        init_screen(&mut buf);

        buf.insert_row(&(0, 0), &['b']);

        assert_eq!(&['b'], buf.rows[0].column());
        assert!(buf.cached());
        assert!(buf.updated());
        assert_eq!(1, buf.history.len());
    }

    #[test]
    fn buffer_insert_row_1() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        init_screen(&mut buf);

        buf.insert_row(&(0, 1), &['b']);

        assert_eq!(&['b'], buf.rows[1].column());
        assert!(buf.cached());
        assert!(buf.updated());
        assert_eq!(1, buf.history.len());
    }

    #[test]
    fn buffer_insert_char_0() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        init_screen(&mut buf);

        buf.insert_char(&(0, 0), 'b');

        assert_eq!(&['b', 'a'], buf.rows[0].column());
        assert!(buf.cached());
        assert!(buf.updated());
        assert_eq!(1, buf.history.len());
    }

    #[test]
    fn buffer_insert_char_1() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        init_screen(&mut buf);

        buf.insert_char(&(1, 0), 'b');

        assert_eq!(&['a', 'b'], buf.rows[0].column());
        assert!(buf.cached());
        assert!(buf.updated());
        assert_eq!(1, buf.history.len());
    }

    #[test]
    fn buffer_insert_char_xoverflow() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        init_screen(&mut buf);

        buf.insert_char(&(2, 0), 'b');

        assert_eq!(&['a'], buf.rows[0].column());
        assert!(!buf.cached());
        assert!(!buf.updated());
        assert_eq!(0, buf.history.len());
    }

    #[test]
    fn buffer_insert_chars_0() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        init_screen(&mut buf);

        buf.insert_chars(&(0, 0), &['b', 'c']);

        assert_eq!(&['b', 'c', 'a'], buf.rows[0].column());
        assert!(buf.cached());
        assert!(buf.updated());
        assert_eq!(1, buf.history.len());
    }

    #[test]
    fn buffer_insert_chars_1() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        init_screen(&mut buf);

        buf.insert_chars(&(1, 0), &['b', 'c']);

        assert_eq!(&['a', 'b', 'c'], buf.rows[0].column());
        assert!(buf.cached());
        assert!(buf.updated());
        assert_eq!(1, buf.history.len());
    }

    #[test]
    fn buffer_insert_chars_xoverflow() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        init_screen(&mut buf);

        buf.insert_chars(&(2, 0), &['b', 'c']);

        assert_eq!(&['a'], buf.rows[0].column());
        assert!(!buf.cached());
        assert!(!buf.updated());
        assert_eq!(0, buf.history.len());
    }

    #[test]
    fn buffer_insert_char_yoverflow() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        init_screen(&mut buf);

        buf.insert_char(&(0, 1), 'b');

        assert_eq!(&['a'], buf.rows[0].column());
        assert!(!buf.cached());
        assert!(!buf.updated());
        assert_eq!(0, buf.history.len());
    }

    #[test]
    fn buffer_rfind_at_0() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a', 'b', 'c']);
        buf.insert_row(&(0, 1), &['a', 'b', 'c']);
        buf.insert_row(&(0, 2), &['a', 'b', 'c']);
        init_screen(&mut buf);

        let at = buf.rfind_at(&(0, 3), "bc");

        assert_eq!(Some((1, 2)), at);
    }

    #[test]
    fn buffer_rfind_at_1() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a', 'b', 'c']);
        buf.insert_row(&(0, 1), &['a', 'b', 'c']);
        buf.insert_row(&(0, 2), &['a', 'b', 'c']);
        init_screen(&mut buf);

        let at = buf.rfind_at(&(1, 1), "bc");

        assert_eq!(Some((1, 0)), at);
    }

    #[test]
    fn buffer_rfind_at_notfound() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a', 'b', 'c']);
        buf.insert_row(&(0, 1), &['a', 'b', 'c']);
        buf.insert_row(&(0, 2), &['a', 'b', 'c']);
        init_screen(&mut buf);

        let at = buf.rfind_at(&(1, 0), "bc");

        assert_eq!(None, at);
    }

    #[test]
    fn buffer_row_char_len() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        init_screen(&mut buf);

        let len = buf.row_char_len(&(0, 0));

        assert_eq!(1, len);
    }

    #[test]
    fn buffer_row_char_len_yoverflow() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        init_screen(&mut buf);

        let len = buf.row_char_len(&(0, 1));

        assert_eq!(0, len);
    }

    #[test]
    fn buffer_rows() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        init_screen(&mut buf);

        let len = buf.rows();

        assert_eq!(1, len);
    }

    #[test]
    fn buffer_paste_pending() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        buf.pending = Some(Row::from(&['b'][..]));
        init_screen(&mut buf);

        buf.paste_pending(&(0, 0));

        assert_eq!(&['b', 'a'], buf.rows[0].column());
        assert!(buf.cached());
        assert!(buf.updated());
        assert_eq!(1, buf.history.len());
    }

    #[test]
    fn buffer_save() {
        let mut buf = Buffer::default();
        buf.set_filename(&PathBuf::from("a.txt"));
        buf.insert_row(&(0, 0), &['a']);
        buf.history.clear();

        let ret = buf.save();

        assert!(ret.is_ok());
        assert!(!buf.cached());
        assert!(buf.updated());
    }

    #[test]
    fn buffer_save_none() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        buf.history.clear();

        let ret = buf.save();

        assert!(ret.is_ok());
        assert!(buf.cached());
        assert!(buf.updated());
    }

    #[test]
    fn buffer_shrink_row() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a', 'b', 'c']);
        init_screen(&mut buf);

        buf.shrink_row(&(1, 0));

        assert_eq!(1, buf.rows());
        assert_eq!(&['a'], buf.rows[0].column());
        assert!(buf.cached());
        assert!(buf.updated());
        assert_eq!(1, buf.history.len());
    }

    #[test]
    fn buffer_shrink_row_yoverflow() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a', 'b', 'c']);
        init_screen(&mut buf);

        buf.shrink_row(&(1, 1));

        assert_eq!(1, buf.rows());
        assert_eq!(&['a', 'b', 'c'], buf.rows[0].column());
        assert!(!buf.cached());
        assert!(!buf.updated());
        assert_eq!(0, buf.history.len());
    }

    #[test]
    fn buffer_split_row() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a', 'b', 'c']);
        init_screen(&mut buf);

        buf.split_row(&(1, 0));

        assert_eq!(2, buf.rows());
        assert_eq!(&['a'], buf.rows[0].column());
        assert_eq!(&['b', 'c'], buf.rows[1].column());
        assert!(buf.cached());
        assert!(buf.updated());
        assert_eq!(1, buf.history.len());
    }

    #[test]
    fn buffer_split_row_start() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a', 'b', 'c']);
        init_screen(&mut buf);

        buf.split_row(&(0, 0));

        assert_eq!(2, buf.rows());
        assert!(buf.rows[0].is_empty());
        assert_eq!(&['a', 'b', 'c'], buf.rows[1].column());
        assert!(buf.cached());
        assert!(buf.updated());
        assert_eq!(1, buf.history.len());
    }

    #[test]
    fn buffer_split_row_end() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a', 'b', 'c']);
        init_screen(&mut buf);

        buf.split_row(&(3, 0));

        assert_eq!(2, buf.rows());
        assert_eq!(&['a', 'b', 'c'], buf.rows[0].column());
        assert!(buf.rows[1].is_empty());
        assert!(buf.cached());
        assert!(buf.updated());
        assert_eq!(1, buf.history.len());
    }

    #[test]
    fn buffer_split_row_yoverflow() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a', 'b', 'c']);
        init_screen(&mut buf);

        buf.split_row(&(1, 1));

        assert_eq!(1, buf.rows());
        assert_eq!(&['a', 'b', 'c'], buf.rows[0].column());
        assert!(!buf.cached());
        assert!(!buf.updated());
        assert_eq!(0, buf.history.len());
    }

    #[test]
    fn buffer_squash_row() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        buf.insert_row(&(0, 1), &['b']);
        init_screen(&mut buf);

        buf.squash_row(&(0, 1));

        assert_eq!(1, buf.rows());
        assert_eq!(&['a', 'b'], buf.rows[0].column());
        assert!(buf.cached());
        assert!(buf.updated());
        assert_eq!(1, buf.history.len());
    }

    #[test]
    fn buffer_squash_start() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        buf.insert_row(&(0, 1), &['b']);
        init_screen(&mut buf);

        buf.squash_row(&(0, 0));

        assert_eq!(2, buf.rows());
        assert_eq!(&['a'], buf.rows[0].column());
        assert_eq!(&['b'], buf.rows[1].column());
        assert!(!buf.cached());
        assert!(!buf.updated());
        assert_eq!(0, buf.history.len());
    }

    #[test]
    fn buffer_squash_row_yoverflow() {
        let mut buf = Buffer::default();
        buf.insert_row(&(0, 0), &['a']);
        buf.insert_row(&(0, 1), &['b']);
        init_screen(&mut buf);

        buf.squash_row(&(0, 2));

        assert_eq!(2, buf.rows());
        assert_eq!(&['a'], buf.rows[0].column());
        assert_eq!(&['b'], buf.rows[1].column());
        assert!(!buf.cached());
        assert!(!buf.updated());
        assert_eq!(0, buf.history.len());
    }

    // -------------------------------------------------------------------------------------------

    #[test]
    fn row_append() {
        let mut buf = Row::default();

        buf.append(&['a']);

        assert_eq!(&['a'], buf.column());
    }

    #[test]
    fn row_insert() {
        let mut buf = Row::default();

        buf.insert(0, 'a');

        assert_eq!(&['a'], buf.column());
    }

    #[test]
    fn row_insert_overflow() {
        let mut buf = Row::default();

        buf.insert(1, 'a');

        assert!(buf.is_empty());
    }

    #[test]
    fn row_insert_slice() {
        let mut buf = Row::default();

        buf.insert_slice(0, &['a', 'b']);

        assert_eq!(&['a', 'b'], buf.column());
    }

    #[test]
    fn row_insert_slice_overflow() {
        let mut buf = Row::default();

        buf.insert_slice(1, &['a', 'b']);

        assert!(buf.is_empty());
    }

    #[test]
    fn row_last_char_width_0() {
        let buf = Row::default();

        assert_eq!(0, buf.last_char_width());
    }

    #[test]
    fn row_last_char_width_1() {
        let buf = Row::from(&['a'][..]);

        assert_eq!(1, buf.last_char_width());
    }

    #[test]
    fn row_last_char_width_2() {
        let buf = Row::from(&['あ'][..]);

        assert_eq!(2, buf.last_char_width());
    }

    #[test]
    fn row_shrink_width_1() {
        let mut buf = Row::from(&['a', 'b', 'c'][..]);

        let removed = buf.shrink_width(1);

        assert_eq!(&['b', 'c'], buf.column());
        assert_eq!(1, removed)
    }

    #[test]
    fn row_shrink_width_2() {
        let mut buf = Row::from(&['あ', 'い', 'う'][..]);

        let removed = buf.shrink_width(2);

        assert_eq!(&['い', 'う'], buf.column());
        assert_eq!(2, removed)
    }

    #[test]
    fn row_shrink_width_3() {
        let mut buf = Row::from(&['あ', 'い', 'う'][..]);

        let removed = buf.shrink_width(3);

        assert_eq!(&['う'], buf.column());
        assert_eq!(4, removed)
    }

    #[test]
    fn row_shrink_width_all() {
        let mut buf = Row::from(&['a', 'b', 'c'][..]);

        let removed = buf.shrink_width(3);

        assert!(buf.is_empty());
        assert_eq!(3, removed)
    }

    #[test]
    fn row_slice_width_0() {
        let buf = Row::from(&['a', 'b', 'c'][..]);

        let render = buf.slice_width(1..1);

        assert!(render.is_empty());
    }

    #[test]
    fn row_slice_width_1() {
        let buf = Row::from(&['a', 'b', 'c'][..]);

        let render = buf.slice_width(1..2);

        assert_eq!(&['b'], render.column());
    }

    #[test]
    fn row_slice_width_2() {
        let buf = Row::from(&['あ', 'い', 'う'][..]);

        let render = buf.slice_width(2..4);

        assert_eq!(&['い'], render.column());
    }

    #[test]
    fn row_slice_width_4() {
        let buf = Row::from(&['あ', 'い', 'う'][..]);

        let render = buf.slice_width(1..5);

        assert_eq!(&[' ', 'い', ' '], render.column());
    }

    #[test]
    fn row_split_off() {
        let mut buf = Row::from(&['a', 'b', 'c'][..]);

        let buf2 = buf.split_off(1);

        assert_eq!(&['a'], buf.column());
        assert_eq!(&['b', 'c'], buf2.column());
    }

    #[test]
    fn row_to_string_at_0() {
        let buf = Row::from(&['a', 'b', 'c'][..]);

        let s = buf.to_string_at(0);

        assert_eq!("abc", s);
    }

    #[test]
    fn row_to_string_at_1() {
        let buf = Row::from(&['a', 'b', 'c'][..]);

        let s = buf.to_string_at(1);

        assert_eq!("bc", s);
    }

    #[test]
    fn row_to_string_at_3() {
        let buf = Row::from(&['a', 'b', 'c'][..]);

        let s = buf.to_string_at(3);

        assert_eq!("", s);
    }

    #[test]
    fn row_truncate_width_0() {
        let mut buf = Row::from(&['a', 'b', 'c'][..]);

        let rest = buf.truncate_width(0);

        assert!(buf.is_empty());
        assert_eq!(0, rest);
    }

    #[test]
    fn row_truncate_width_1() {
        let mut buf = Row::from(&['a', 'b', 'c'][..]);

        let rest = buf.truncate_width(1);

        assert_eq!(&['a'], buf.column());
        assert_eq!(1, rest);
    }

    #[test]
    fn row_truncate_width_2() {
        let mut buf = Row::from(&['あ', 'い', 'う'][..]);

        let rest = buf.truncate_width(3);

        assert_eq!(&['あ'], buf.column());
        assert_eq!(2, rest);
    }

    #[test]
    fn row_remove() {
        let mut buf = Row::from(&['a', 'b'][..]);

        buf.remove(0);

        assert_eq!(&['b'], buf.column());
    }

    #[test]
    fn row_remove_overflow() {
        let mut buf = Row::from(&['a', 'b'][..]);

        buf.remove(2);

        assert_eq!(&['a', 'b'], buf.column());
    }

    #[test]
    fn row_remove_range_0() {
        let mut buf = Row::from(&['a', 'b', 'c'][..]);

        buf.remove_range(0..2);

        assert_eq!(&['c'], buf.column());
    }

    #[test]
    fn row_remove_range_1() {
        let mut buf = Row::from(&['a', 'b', 'c'][..]);

        buf.remove_range(1..2);

        assert_eq!(&['a', 'c'], buf.column());
    }

    #[test]
    fn row_remove_range_2() {
        let mut buf = Row::from(&['a', 'b', 'c'][..]);

        buf.remove_range(2..3);

        assert_eq!(&['a', 'b'], buf.column());
    }

    #[test]
    fn row_remove_range_overflow() {
        let mut buf = Row::from(&['a', 'b', 'c'][..]);

        buf.remove_range(0..4);

        assert_eq!(&['a', 'b', 'c'], buf.column());
    }

    #[test]
    fn row_rev_at_0() {
        let buf = Row::from(&['a', 'b', 'c'][..]);

        let rev = buf.rev_at(0);

        assert!(rev.is_empty());
    }

    #[test]
    fn row_rev_at_1() {
        let buf = Row::from(&['a', 'b', 'c'][..]);

        let rev = buf.rev_at(1);

        assert_eq!(&['a'], rev.column());
    }

    #[test]
    fn row_rev_at_2() {
        let buf = Row::from(&['a', 'b', 'c'][..]);

        let rev = buf.rev_at(2);

        assert_eq!(&['b', 'a'], rev.column());
    }

    #[test]
    fn row_width_1() {
        let buf = Row::from(&['a', 'b', 'c'][..]);

        assert_eq!(3, buf.width());
    }

    #[test]
    fn row_width_2() {
        let buf = Row::from(&['あ', 'い', 'う'][..]);

        assert_eq!(6, buf.width());
    }

    #[test]
    fn row_width_range_1() {
        let buf = Row::from(&['a', 'b', 'c'][..]);

        assert_eq!(2, buf.width_range(0..2));
    }

    #[test]
    fn row_width_range_2() {
        let buf = Row::from(&['あ', 'い', 'う'][..]);

        assert_eq!(4, buf.width_range(0..2));
    }
}
