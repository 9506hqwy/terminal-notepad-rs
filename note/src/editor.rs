use crate::buffer::Buffer;
use crate::cursor::{AsCoordinates, Coordinates, Cursor};
use crate::error::Error;
use crate::key_event::{Event, KeyEvent, WindowEvent};
use crate::prompt::{self, Prompt};
use crate::screen::{MessageBar, Screen, StatusBar};
use crate::terminal::Terminal;
use std::path::{Path, PathBuf};
use std::process::exit;

const TEXT_CONFIRM_KILL_BUFFER: &str = "Buffer is modified. Kill buffer (y/N) : ";

const TEXT_MESSAGE_INPUT_FILENAME: &str = "Filename (ESC:quit): ";
const TEXT_MESSAGE_INPUT_KEYWORD: &str = "Input keyword (ESC:quit F3:next S+F3:prev): ";
const TEXT_MESSAGE_MENU: &str = "^Q:Quit ^S:Save ^F:Find";

pub struct Editor<T: Terminal> {
    cursor: Cursor,
    cursor_modified: bool,
    content: Buffer,
    content_modified: bool,
    terminal: T,
    screen: Screen,
    screen_modified: bool,
    status: StatusBar,
    status_modified: bool,
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
            cursor_modified: false,
            content,
            content_modified: false,
            terminal,
            screen,
            screen_modified: false,
            status,
            status_modified: false,
            message,
        })
    }

    pub fn confirm_exit(&mut self) -> Result<bool, Error> {
        let mut prompt =
            prompt::YesNo::new(TEXT_CONFIRM_KILL_BUFFER, &self.screen, &mut self.terminal);
        prompt.confirm()
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
                self.content_modified = true;

                let m1 = self.cursor.move_up(&self.content);
                let m2 = self.cursor.set_x(&self.content, x);
                m1 || m2
            }
            _ => {
                self.content.delete_char(&self.cursor);
                self.content_modified = true;

                self.cursor.move_left(&self.content)
            }
        }
    }

    pub fn enter(&mut self) -> bool {
        self.content.split_row(&self.cursor);
        self.content_modified = true;

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
        self.screen_modified = true;
        self.status_modified = true;

        let ret;
        let moved;
        let src;
        {
            let mut prompt = prompt::FindKeyword::new(
                TEXT_MESSAGE_INPUT_KEYWORD,
                &mut self.cursor,
                &self.content,
                &mut self.screen,
                &mut self.status,
                &mut self.terminal,
            );

            ret = prompt.handle_events()?;
            moved = prompt.source() != prompt.current();
            src = prompt.source().as_coordinates();
        }

        if ret.is_none() {
            self.cursor.set(&self.content, &src);
        }

        Ok(moved)
    }

    pub fn handle_events(&mut self) -> Result<bool, Error> {
        self.content_modified = false;
        self.screen_modified = false;
        self.status_modified = false;
        self.cursor_modified = match T::read_event_timeout()? {
            Event::Key(KeyEvent::BackSpace, _) => self.delete_char(),
            Event::Key(KeyEvent::Enter, _) => self.enter(),
            Event::Key(KeyEvent::End, _) => self.cursor.move_to_xmax(&self.content),
            Event::Key(KeyEvent::PageUp, _) => {
                self.screen_modified = self.screen.move_up();
                self.cursor.move_up_screen(&self.content, &self.screen)
            }
            Event::Key(KeyEvent::PageDown, _) => {
                self.screen_modified = self.screen.move_down(&self.content);
                self.cursor.move_down_screen(&self.content, &self.screen)
            }
            Event::Key(KeyEvent::Home, _) => self.cursor.move_to_x0(),
            Event::Key(KeyEvent::ArrowLeft, _) => self.cursor.move_left(&self.content),
            Event::Key(KeyEvent::ArrowUp, _) => self.cursor.move_up(&self.content),
            Event::Key(KeyEvent::ArrowRight, _) => self.cursor.move_right(&self.content),
            Event::Key(KeyEvent::ArrowDown, _) => self.cursor.move_down(&self.content),
            Event::Key(KeyEvent::Delete, _) => {
                let moved = self.cursor.move_right(&self.content);
                self.delete_char();
                moved
            }
            Event::Key(KeyEvent::DeleteRow, _) => {
                if self.content.row_char_len(&self.cursor) == 0 {
                    self.content.delete_row(&self.cursor);
                } else {
                    self.content.shrink_row(&self.cursor);
                }
                self.content_modified = true;
                // FIXEDME: ???
                true
            }
            Event::Key(KeyEvent::Find, _) => self.find()?,
            Event::Key(KeyEvent::Exit, _) => {
                self.exit()?;
                false
            }
            Event::Key(KeyEvent::Save, _) => {
                self.save()?;
                false
            }
            Event::Key(KeyEvent::Char(ch), _) if !ch.is_ascii_control() => self.input_char(ch),
            Event::Window(WindowEvent::Resize) => {
                self.resize_screen()?;
                false
            }
            _ => false,
        };
        Ok(self.cursor_modified)
    }

    pub fn input_char(&mut self, ch: char) -> bool {
        match self.cursor.as_coordinates() {
            (_, y) if self.content.rows() <= y => self.content.insert_row(&self.cursor, &[ch]),
            _ => self.content.insert_char(&self.cursor, ch),
        }
        self.content_modified = true;

        self.cursor.move_right(&self.content)
    }

    pub fn init(&mut self) -> Result<(), Error> {
        self.screen.draw(&self.content, &mut self.terminal)?;
        self.status.draw(&self.cursor, &mut self.terminal)?;
        self.message.draw(&mut self.terminal)?;

        self.terminal.set_cursor_position(0, 0)?;

        Ok(())
    }

    pub fn refresh(&mut self) -> Result<(), Error> {
        let render = self.cursor.render(&self.content);

        if self.screen.fit(&self.content, &render) || self.content_modified || self.screen_modified
        {
            self.screen.clear(&mut self.terminal)?;
            self.screen.draw(&self.content, &mut self.terminal)?;
        }

        if self.cursor_modified || self.status_modified {
            self.status.draw(&render, &mut self.terminal)?;
        }

        self.message.draw(&mut self.terminal)?;

        self.terminal.set_cursor_position(
            render.x() - self.screen.left(),
            render.y() - self.screen.top(),
        )?;

        Ok(())
    }

    pub fn resize_screen(&mut self) -> Result<(), Error> {
        let (width, height) = self.terminal.get_screen_size()?;

        if self.screen.width() != width || self.screen.height() != height {
            self.content_modified = true;

            self.screen.resize(height, width);
            self.screen_modified = true;

            self.status.resize(&self.screen);
            self.status_modified = true;

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

            if let Some(filename) = prompt.handle_events()? {
                let path = PathBuf::from(filename);
                self.content.save_as(&path)?;
                self.content.set_filename(&path);
                self.status
                    .set_filename(path.file_name().and_then(|n| n.to_str()).unwrap());
                self.status_modified = true;
            }
        }

        Ok(())
    }

    pub fn screen(&self) -> &Screen {
        &self.screen
    }
}
