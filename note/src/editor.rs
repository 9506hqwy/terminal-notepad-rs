use crate::buffer::{Buffer, Row};
use crate::cursor::{AsCoordinates, Coordinates, Cursor};
use crate::error::Error;
use crate::key_event::{Event, KeyEvent, KeyModifier, WindowEvent};
use crate::prompt::{self, Prompt};
use crate::screen::{MessageBar, Screen, StatusBar, refresh_screen, resize_screen};
use crate::terminal::Terminal;
use std::cmp::{max, min};
use std::path::{Path, PathBuf};
use std::process::exit;

const TEXT_CONFIRM_KILL_BUFFER: &str = "Buffer is modified. Kill buffer (y/N) : ";

const TEXT_MESSAGE_INPUT_FILENAME: &str = "Filename (ESC:quit): ";
const TEXT_MESSAGE_INPUT_KEYWORD: &str = "Input keyword (ESC:quit F3:next S+F3:prev): ";
const TEXT_MESSAGE_INPUT_LINENO: &str = "Go to line (ESC:quit): ";
const TEXT_MESSAGE_INPUT_REPLACE: &str = "Replace word (ESC:quit): ";
const TEXT_MESSAGE_MENU: &str = "^Q:Quit ^S:Save ^F:Find";

pub struct Editor<T: Terminal> {
    cursor: Cursor,
    content: Buffer,
    terminal: T,
    screen: Screen,
    select: Select,
    status: StatusBar,
    message: MessageBar,
}

impl<T: Terminal> Editor<T> {
    pub fn new(filename: Option<&Path>, terminal: T) -> Result<Self, Error> {
        let content = Buffer::try_from(filename)?;
        let screen = Screen::current(&terminal)?;
        let status = StatusBar::new(&screen, filename.and_then(|f| f.to_str()));
        let message = MessageBar::new(&screen, TEXT_MESSAGE_MENU);

        Ok(Editor {
            cursor: Cursor::default(),
            content,
            terminal,
            screen,
            select: Select::default(),
            status,
            message,
        })
    }

    pub fn confirm_exit(&mut self) -> Result<bool, Error> {
        let mut prompt = prompt::YesNo::new(
            &mut self.cursor,
            &mut self.content,
            &mut self.screen,
            &mut self.status,
            &mut self.message,
            &mut self.terminal,
        );
        let ret = prompt.confirm(TEXT_CONFIRM_KILL_BUFFER)?;
        self.message.force_update();
        Ok(ret)
    }

    pub fn content(&self) -> &Buffer {
        &self.content
    }

    pub fn cursor(&self) -> &Cursor {
        &self.cursor
    }

    pub fn delete_char(&mut self) -> bool {
        match self.cursor.as_coordinates() {
            (0, 0) => false,
            (0, y) if self.content.rows() <= y => false,
            (0, y) => {
                let mut at = self.cursor.clone();
                at.set_y(&self.content, y - 1);

                let x = self.content.row_char_len(&at);
                self.content.squash_row(&self.cursor);

                let m1 = self.cursor.move_up(&self.content);
                let m2 = self.cursor.set_x(&self.content, x);
                m1 || m2
            }
            _ => {
                self.content.delete_char(&self.cursor);
                self.cursor.move_left(&self.content)
            }
        }
    }

    pub fn enter(&mut self) -> bool {
        self.content.split_row(&self.cursor);

        let m1 = self.cursor.move_to_x0();
        let m2 = self.cursor.move_down(&self.content);
        m1 || m2
    }

    pub fn exit(&mut self) -> Result<(), Error> {
        if self.content.cached() && !self.confirm_exit()? {
            return Ok(());
        }

        exit(0);
    }

    pub fn find(&mut self) -> Result<bool, Error> {
        let ret;
        let moved;
        let src;
        {
            let row = self.get_selected_text().and_then(|s| s.first().cloned());
            self.select.disable();

            let mut prompt = prompt::FindKeyword::new(
                &mut self.cursor,
                &mut self.content,
                &mut self.screen,
                &mut self.status,
                &mut self.message,
                &mut self.terminal,
            );

            ret = prompt.handle_events(
                TEXT_MESSAGE_INPUT_KEYWORD,
                row.map(|r| r.to_string_at(0)).as_deref(),
            )?;
            moved = prompt.source() != prompt.cursor();
            src = prompt.source().as_coordinates();
        }

        if ret.is_none() {
            self.cursor.set(&self.content, &src);
        }

        // Delete text decoration.
        self.screen.force_update();

        self.message.force_update();

        Ok(moved)
    }

    pub fn goto(&mut self) -> Result<bool, Error> {
        let rows = self.content.rows();

        let mut prompt = prompt::Input::new(
            &mut self.cursor,
            &mut self.content,
            &mut self.screen,
            &mut self.status,
            &mut self.message,
            &mut self.terminal,
        );

        while let Some(lineno) = prompt.handle_events(TEXT_MESSAGE_INPUT_LINENO, None)? {
            if let Ok(lineno) = lineno.parse::<usize>() {
                if 0 < lineno && lineno <= rows {
                    let cur = self.cursor.clone();
                    self.cursor.set_y(&self.content, lineno - 1);
                    self.message.force_update();
                    return Ok(cur != self.cursor);
                }
            }
        }

        self.message.force_update();
        Ok(false)
    }

    pub fn handle_events(&mut self) -> Result<(), Error> {
        let event = T::read_event_timeout()?;
        match event {
            Event::Key(KeyEvent::BackSpace, _) => {
                self.delete_char();
            }
            Event::Key(KeyEvent::Enter, _) => {
                self.enter();
            }
            Event::Key(KeyEvent::End, _) => {
                self.cursor.move_to_xmax(&self.content);
            }
            Event::Key(KeyEvent::PageUp, _) => {
                self.screen.move_up();
                self.cursor.move_up_screen(&self.content, &self.screen);
            }
            Event::Key(KeyEvent::PageDown, _) => {
                self.screen.move_down(&self.content);
                self.cursor.move_down_screen(&self.content, &self.screen);
            }
            Event::Key(KeyEvent::Home, _) => {
                self.cursor.move_to_x0();
            }
            Event::Key(KeyEvent::ArrowLeft, _) => {
                self.cursor.move_left(&self.content);
            }
            Event::Key(KeyEvent::ArrowUp, _) => {
                self.cursor.move_up_render(&self.content);
            }
            Event::Key(KeyEvent::ArrowRight, _) => {
                self.cursor.move_right(&self.content);
            }
            Event::Key(KeyEvent::ArrowDown, _) => {
                self.cursor.move_down_render(&self.content);
            }
            Event::Key(KeyEvent::Delete, _) => {
                self.cursor.move_right(&self.content);
                self.delete_char();
            }
            Event::Key(KeyEvent::DeleteRow, _) => {
                if self.content.row_char_len(&self.cursor) == 0 {
                    self.content.delete_row(&self.cursor);
                } else {
                    self.content.shrink_row(&self.cursor);
                }
            }
            Event::Key(KeyEvent::Copy, _) => {
                if let (Some(start), Some(end)) = (self.select.start(), self.select.end()) {
                    self.content.copy_pending(start..end, self.select.mode());
                }
            }
            Event::Key(KeyEvent::Cut, _) => {
                if let (Some(start), Some(end)) = (self.select.start(), self.select.end()) {
                    self.content.delete_chars(start, end, self.select.mode());
                    self.cursor.set(&self.content, start);
                }
            }
            Event::Key(KeyEvent::Find, _) => {
                self.find()?;
            }
            Event::Key(KeyEvent::Exit, _) => {
                self.exit()?;
            }
            Event::Key(KeyEvent::Goto, _) => {
                self.goto()?;
            }
            Event::Key(KeyEvent::Save, _) => {
                self.save()?;
            }
            Event::Key(KeyEvent::Paste, _) => {
                if self.content.pending().is_some() {
                    if let Some(pos) = self.content.paste_pending(&self.cursor) {
                        self.cursor.set(&self.content, &pos);
                    }
                }
            }
            Event::Key(KeyEvent::Replace, _) => self.replace()?,
            Event::Key(KeyEvent::Undo, _) => {
                if let Some(cur) = self.content.undo() {
                    self.cursor.set(&self.content, &cur);
                }
            }
            Event::Key(KeyEvent::Char(ch), _) if !ch.is_ascii_control() => {
                self.input_char(ch);
            }
            Event::Window(WindowEvent::Resize) => {
                self.resize_screen()?;
            }
            _ => {}
        };

        self.update_select(event);
        Ok(())
    }

    pub fn input_char(&mut self, ch: char) -> bool {
        match self.cursor.as_coordinates() {
            (_, y) if self.content.rows() <= y => self.content.insert_row(&self.cursor, &[ch]),
            _ => self.content.insert_char(&self.cursor, ch),
        }

        self.cursor.move_right(&self.content)
    }

    pub fn init(&mut self) -> Result<(), Error> {
        refresh_screen(
            &self.cursor,
            &mut self.content,
            &mut self.screen,
            &mut self.select,
            &mut self.status,
            &mut self.message,
            &mut self.terminal,
        )?;

        self.terminal.set_cursor_position(0, 0)?;

        Ok(())
    }

    pub fn refresh(&mut self) -> Result<(), Error> {
        let render = self.cursor.render(&self.content);

        self.screen.fit(&self.content, &render);

        refresh_screen(
            &render,
            &mut self.content,
            &mut self.screen,
            &mut self.select,
            &mut self.status,
            &mut self.message,
            &mut self.terminal,
        )?;

        self.terminal.set_cursor_position(
            render.x() - self.screen.left(),
            render.y() - self.screen.top(),
        )?;

        Ok(())
    }

    pub fn replace(&mut self) -> Result<(), Error> {
        let row = self.get_selected_text().and_then(|s| s.first().cloned());
        self.select.disable();

        let mut prompt = prompt::Replace::new(
            &mut self.cursor,
            &mut self.content,
            &mut self.screen,
            &mut self.status,
            &mut self.message,
            &mut self.terminal,
        );
        prompt.replace(
            TEXT_MESSAGE_INPUT_REPLACE,
            row.map(|r| r.to_string_at(0)).as_deref(),
        )?;

        // Delete text decoration.
        self.screen.force_update();

        self.message.force_update();
        Ok(())
    }

    pub fn resize_screen(&mut self) -> Result<(), Error> {
        resize_screen(
            &mut self.screen,
            &mut self.status,
            &mut self.message,
            &mut self.terminal,
        )
    }

    pub fn save(&mut self) -> Result<(), Error> {
        self.content.save()?;

        if self.content.cached() {
            let mut prompt = prompt::Input::new(
                &mut self.cursor,
                &mut self.content,
                &mut self.screen,
                &mut self.status,
                &mut self.message,
                &mut self.terminal,
            );

            if let Some(filename) = prompt.handle_events(TEXT_MESSAGE_INPUT_FILENAME, None)? {
                let path = PathBuf::from(filename);
                self.content.save_as(&path)?;
                self.content.set_filename(&path);
                self.status
                    .set_filename(path.file_name().and_then(|n| n.to_str()).unwrap());
            }

            self.message.force_update();
        }

        Ok(())
    }

    pub fn select(&self) -> &Select {
        &self.select
    }

    pub fn screen(&self) -> &Screen {
        &self.screen
    }

    fn get_selected_text(&self) -> Option<Vec<Row>> {
        if let (Some(start), Some(end)) = (self.select.start(), self.select.end()) {
            self.content.get_range(start..end, self.select.mode())
        } else {
            None
        }
    }

    fn update_select(&mut self, event: Event) {
        if let Event::Key(e, m) = event {
            if selected_moved(m) && row_moved(e) {
                if self.select.enabled {
                    self.select.set_end(&self.cursor);
                } else {
                    self.select.set_start(&self.cursor, SelectMode::from(m));
                }
            } else {
                self.select.disable();
            }
        } else {
            self.select.disable();
        }
    }
}

// -----------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum SelectMode {
    #[default]
    None,
    Rectangle,
}

impl From<KeyModifier> for SelectMode {
    fn from(value: KeyModifier) -> SelectMode {
        match value {
            KeyModifier::CtrlLeft => SelectMode::Rectangle,
            _ => SelectMode::None,
        }
    }
}

// -----------------------------------------------------------------------------------------------

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Select {
    mode: SelectMode,
    range: Option<(Cursor, Cursor)>,
    previous: Option<Cursor>,
    enabled: bool,
    updated: bool,
}

impl Select {
    pub fn clear_updated(&mut self) {
        self.updated = false;
    }

    pub fn disable(&mut self) {
        let cur = self.clone();

        if !self.enabled {
            self.mode = SelectMode::None;
            self.range = None;
            self.previous = None;
        }

        self.enabled = false;

        self.updated |= cur != *self;
    }

    pub fn mode(&self) -> SelectMode {
        self.mode
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn end(&self) -> Option<&Cursor> {
        if let (Some(s), Some(e)) = (
            self.range.as_ref().map(|r| &r.0),
            self.range.as_ref().map(|r| &r.1),
        ) {
            if s.y() < e.y() {
                Some(e)
            } else if e.y() < s.y() {
                Some(s)
            } else if s.x() < e.x() {
                Some(e)
            } else {
                Some(s)
            }
        } else {
            None
        }
    }

    pub fn changes(&self, y: usize) -> bool {
        match (self.start(), self.end()) {
            (Some(start), Some(end)) => {
                if start.y() <= y && y <= end.y() {
                    true
                } else if let Some(prev) = &self.previous {
                    prev.y() == y
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    pub fn set_end(&mut self, end: &Cursor) {
        let cur = self.clone();

        self.previous = self.range.as_ref().map(|r| r.1.clone());
        self.range.as_mut().unwrap().1 = end.clone();

        self.updated |= cur != *self;
    }

    pub fn set_start(&mut self, start: &Cursor, mode: SelectMode) {
        let cur = self.clone();

        self.mode = mode;
        self.previous = None;
        self.range = Some((start.clone(), start.clone()));
        self.enabled = true;

        self.updated |= cur != *self;
    }

    pub fn start(&self) -> Option<&Cursor> {
        if let (Some(s), Some(e)) = (
            self.range.as_ref().map(|r| &r.0),
            self.range.as_ref().map(|r| &r.1),
        ) {
            if s.y() < e.y() {
                Some(s)
            } else if e.y() < s.y() {
                Some(e)
            } else if s.x() < e.x() {
                Some(s)
            } else {
                Some(e)
            }
        } else {
            None
        }
    }

    pub fn updated(&self) -> bool {
        self.updated
    }

    pub fn xrange(&self, y: usize) -> Option<(usize, usize)> {
        if !self.enabled {
            return None;
        }

        match self.mode {
            SelectMode::None => self.xrange_none(y),
            SelectMode::Rectangle => self.xrange_rectangle(y),
        }
    }

    fn xrange_none(&self, y: usize) -> Option<(usize, usize)> {
        match (self.start(), self.end()) {
            (Some(start), Some(end)) => {
                if y < start.y() {
                    None
                } else if y == start.y() {
                    if start.y() == end.y() {
                        Some((start.x(), end.x()))
                    } else {
                        Some((start.x(), usize::MAX))
                    }
                } else if end.y() == y {
                    Some((0, end.x()))
                } else if end.y() < y {
                    None
                } else {
                    Some((0, usize::MAX))
                }
            }
            _ => None,
        }
    }

    fn xrange_rectangle(&self, y: usize) -> Option<(usize, usize)> {
        match (self.start(), self.end()) {
            (Some(start), Some(end)) => {
                if start.y() <= y && y <= end.y() {
                    let s = min(start.x(), end.x());
                    let e = max(start.x(), end.x());
                    Some((s, e))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

// -----------------------------------------------------------------------------------------------

fn row_moved(key: KeyEvent) -> bool {
    key == KeyEvent::ArrowLeft
        || key == KeyEvent::ArrowUp
        || key == KeyEvent::ArrowRight
        || key == KeyEvent::ArrowDown
        || key == KeyEvent::End
        || key == KeyEvent::Home
        || key == KeyEvent::Char('\0')
}

fn selected_moved(key: KeyModifier) -> bool {
    key == KeyModifier::CtrlLeft || key == KeyModifier::Shift
}
