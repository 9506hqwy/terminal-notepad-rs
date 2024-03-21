use crate::buffer::{Buffer, Row};
use crate::cursor::{AsCoordinates, Coordinates, Cursor};
use crate::error::Error;
use crate::key_event::{Event, KeyEvent, KeyModifier, WindowEvent};
use crate::prompt::{self, Prompt};
use crate::screen::{MessageBar, Screen, StatusBar};
use crate::terminal::Terminal;
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
        let mut prompt =
            prompt::YesNo::new(TEXT_CONFIRM_KILL_BUFFER, &self.screen, &mut self.terminal);
        let ret = prompt.confirm()?;
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
            let row = self.get_selected_text();
            self.select.disable();

            let mut prompt = prompt::FindKeyword::new(
                TEXT_MESSAGE_INPUT_KEYWORD,
                &mut self.cursor,
                &self.content,
                &mut self.screen,
                &mut self.status,
                &mut self.terminal,
            );

            ret = prompt.handle_events(row.map(|r| r.to_string_at(0)).as_deref())?;
            moved = prompt.source() != prompt.current();
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
        let mut prompt =
            prompt::Input::new(TEXT_MESSAGE_INPUT_LINENO, &self.screen, &mut self.terminal);

        while let Some(lineno) = prompt.handle_events(None)? {
            if let Ok(lineno) = lineno.parse::<usize>() {
                if 0 < lineno && lineno <= self.content.rows() {
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
                if !self.select.enabled() || self.cursor.x() != 0 {
                    self.cursor.move_left(&self.content);
                }
            }
            Event::Key(KeyEvent::ArrowUp, _) => {
                self.cursor.move_up(&self.content);
            }
            Event::Key(KeyEvent::ArrowRight, _) => {
                if !self.select.enabled()
                    || self.cursor.x() != self.content.row_char_len(&self.cursor)
                {
                    self.cursor.move_right(&self.content);
                }
            }
            Event::Key(KeyEvent::ArrowDown, _) => {
                self.cursor.move_down(&self.content);
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
                    self.content.copy_pending(start..end);
                }
            }
            Event::Key(KeyEvent::Cut, _) => {
                if let (Some(start), Some(end)) = (self.select.start(), self.select.end()) {
                    let length = end.x() - start.x();
                    self.content.delete_chars(start, length);
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
                if let Some(row_len) = self.content.pending().map(|r| r.len()) {
                    self.content.paste_pending(&self.cursor);
                    self.cursor.set_x(&self.content, self.cursor.x() + row_len);
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
        self.screen
            .draw(&self.content, &self.select, &mut self.terminal)?;
        self.content.clear_updated();
        self.select.clear_updated();

        self.status.set_cursor(&self.cursor);
        self.status.draw(&mut self.terminal)?;

        self.message.draw(&mut self.terminal)?;

        self.terminal.set_cursor_position(0, 0)?;

        Ok(())
    }

    pub fn refresh(&mut self) -> Result<(), Error> {
        let render = self.cursor.render(&self.content);

        self.screen.fit(&self.content, &render);
        self.screen
            .draw(&self.content, &self.select, &mut self.terminal)?;
        self.content.clear_updated();
        self.select.clear_updated();

        self.status.set_cursor(&render);
        self.status.draw(&mut self.terminal)?;

        self.message.draw(&mut self.terminal)?;

        self.terminal.set_cursor_position(
            render.x() - self.screen.left(),
            render.y() - self.screen.top(),
        )?;

        Ok(())
    }

    pub fn replace(&mut self) -> Result<(), Error> {
        let row = self.get_selected_text();
        self.select.disable();

        let mut prompt = prompt::Replace::new(
            TEXT_MESSAGE_INPUT_REPLACE,
            &mut self.cursor,
            &mut self.content,
            &mut self.screen,
            &mut self.status,
            &mut self.terminal,
        );
        prompt.replace(row.map(|r| r.to_string_at(0)).as_deref())?;

        // Delete text decoration.
        self.screen.force_update();

        self.message.force_update();
        Ok(())
    }

    pub fn resize_screen(&mut self) -> Result<(), Error> {
        let (width, height) = self.terminal.get_screen_size()?;

        if self.screen.width() != width || self.screen.height() != height {
            self.screen.resize(height, width);
            self.status.resize(&self.screen);
            self.message.resize(&self.screen);
        }

        Ok(())
    }

    pub fn save(&mut self) -> Result<(), Error> {
        self.content.save()?;

        if self.content.cached() {
            let mut prompt = prompt::Input::new(
                TEXT_MESSAGE_INPUT_FILENAME,
                &self.screen,
                &mut self.terminal,
            );

            if let Some(filename) = prompt.handle_events(None)? {
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

    fn get_selected_text(&self) -> Option<Row> {
        if let (Some(start), Some(end)) = (self.select.start(), self.select.end()) {
            self.content.get_range(start..end)
        } else {
            None
        }
    }

    fn update_select(&mut self, event: Event) {
        if let Event::Key(e, m) = event {
            // TODO: multi rows support.
            if m == KeyModifier::Shift && row_moved(e) {
                if self.select.enabled {
                    self.select.set_end(&self.cursor);
                } else {
                    self.select.set_start(&self.cursor);
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

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Select {
    range: Option<(Cursor, Cursor)>,
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
            self.range = None;
        }

        self.enabled = false;

        self.updated |= cur != *self;
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn end(&self) -> Option<&Cursor> {
        if let (Some(s), Some(e)) = (
            self.range.as_ref().map(|r| &r.0),
            self.range.as_ref().map(|r| &r.1),
        ) {
            if s.x() < e.x() {
                Some(e)
            } else {
                Some(s)
            }
        } else {
            None
        }
    }

    pub fn in_range(&self, y: usize) -> bool {
        match &self.range {
            Some(range) => range.0.y() <= y && y <= range.1.y(),
            _ => false,
        }
    }

    pub fn set_end(&mut self, end: &Cursor) {
        let cur = self.clone();

        self.range.as_mut().unwrap().1 = end.clone();

        self.updated |= cur != *self;
    }

    pub fn set_start(&mut self, start: &Cursor) {
        let cur = self.clone();

        self.range = Some((start.clone(), start.clone()));
        self.enabled = true;

        self.updated |= cur != *self;
    }

    pub fn start(&self) -> Option<&Cursor> {
        if let (Some(s), Some(e)) = (
            self.range.as_ref().map(|r| &r.0),
            self.range.as_ref().map(|r| &r.1),
        ) {
            if s.x() < e.x() {
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
}

// -----------------------------------------------------------------------------------------------

fn row_moved(key: KeyEvent) -> bool {
    key == KeyEvent::ArrowLeft
        || key == KeyEvent::ArrowRight
        || key == KeyEvent::End
        || key == KeyEvent::Home
        || key == KeyEvent::Char('\0')
}
