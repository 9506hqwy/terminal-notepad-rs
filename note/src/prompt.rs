use crate::buffer::{Buffer, Row};
use crate::cursor::{Coordinates, Cursor};
use crate::editor::Select;
use crate::error::Error;
use crate::key_event::{Event, KeyEvent, KeyModifier};
use crate::screen::{MessageBar, Screen, StatusBar};
use crate::terminal::Terminal;
use crate::Color;
use std::cmp::min;

pub enum KeyInput {
    Ok,
    Continue,
    Cancel,
}

pub trait Prompt<T: Terminal> {
    #[allow(unused_variables)]
    fn callback_event(&mut self, event: &Event, chars: &mut Row) -> Result<(), Error> {
        Ok(())
    }

    fn handle_events(&mut self, value: Option<&str>) -> Result<Option<String>, Error> {
        let mut prompt = MessageBar::new(self.screen(), self.message());
        prompt.set_fg_color(Color::Cyan);

        prompt.draw(self.terminal())?;
        let (prompt_x, prompt_y) = self.terminal().get_cursor_position()?;

        let mut chars = value.map(Row::from).unwrap_or_default();
        chars.truncate_width(self.screen().width() - prompt_x - 1);
        self.terminal()
            .write(prompt_x, prompt_y, chars.column(), Color::White, false)?;

        let mut event = self.read_event_timeout()?;
        while match event {
            Event::Key(KeyEvent::BackSpace, _) => {
                if !chars.is_empty() {
                    chars.remove(chars.len() - 1);
                    match self.handle_input_event(chars.column())? {
                        KeyInput::Ok => false,
                        KeyInput::Continue => true,
                        KeyInput::Cancel => return Ok(None),
                    }
                } else {
                    true
                }
            }
            Event::Key(KeyEvent::Enter, _) => false,
            Event::Key(KeyEvent::Escape, _) => return Ok(None),
            Event::Key(KeyEvent::Char(ch), _) if !ch.is_ascii_control() => {
                chars.insert(chars.len(), ch);
                match self.handle_input_event(chars.column())? {
                    KeyInput::Ok => false,
                    KeyInput::Continue => true,
                    KeyInput::Cancel => return Ok(None),
                }
            }
            Event::Key(..) => match self.handle_event(&event, chars.column())? {
                KeyInput::Ok => false,
                KeyInput::Continue => true,
                KeyInput::Cancel => return Ok(None),
            },
            // TODO: resize screen
            _ => true,
        } {
            self.callback_event(&event, &mut chars)?;

            prompt.draw(self.terminal())?;
            chars.truncate_width(self.screen().width() - prompt_x - 1);
            self.terminal()
                .write(prompt_x, prompt_y, chars.column(), Color::White, false)?;
            event = self.read_event_timeout()?;
        }

        Ok(Some(chars.to_string_at(0)))
    }

    #[allow(unused_variables)]
    fn handle_event(&mut self, event: &Event, chars: &[char]) -> Result<KeyInput, Error> {
        Ok(KeyInput::Continue)
    }

    #[allow(unused_variables)]
    fn handle_input_event(&mut self, chars: &[char]) -> Result<KeyInput, Error> {
        Ok(KeyInput::Continue)
    }

    fn message(&self) -> &str;

    fn read_event_timeout(&self) -> Result<Event, Error> {
        T::read_event_timeout()
    }

    fn screen(&self) -> &Screen;

    fn terminal(&mut self) -> &mut T;
}

// -----------------------------------------------------------------------------------------------

pub struct Input<'a, T: Terminal> {
    message: String,
    screen: &'a Screen,
    terminal: &'a mut T,
}

impl<'a, T: Terminal> Prompt<T> for Input<'a, T> {
    fn message(&self) -> &str {
        self.message.as_str()
    }

    fn screen(&self) -> &Screen {
        self.screen
    }

    fn terminal(&mut self) -> &mut T {
        self.terminal
    }
}

impl<'a, T: Terminal> Input<'a, T> {
    pub fn new(message: &str, screen: &'a Screen, terminal: &'a mut T) -> Self {
        Input {
            message: message.to_string(),
            screen,
            terminal,
        }
    }
}

// -----------------------------------------------------------------------------------------------

pub struct YesNo<'a, T: Terminal> {
    message: String,
    screen: &'a Screen,
    terminal: &'a mut T,
}

impl<'a, T: Terminal> Prompt<T> for YesNo<'a, T> {
    fn message(&self) -> &str {
        self.message.as_str()
    }

    fn screen(&self) -> &Screen {
        self.screen
    }

    fn terminal(&mut self) -> &mut T {
        self.terminal
    }
}

impl<'a, T: Terminal> YesNo<'a, T> {
    pub fn new(message: &str, screen: &'a Screen, terminal: &'a mut T) -> Self {
        YesNo {
            message: message.to_string(),
            screen,
            terminal,
        }
    }

    pub fn confirm(&mut self) -> Result<bool, Error> {
        while let Some(yes_no) = self.handle_events(None)? {
            let answer = yes_no.to_ascii_lowercase();

            if answer == "y" || answer == "yes" {
                return Ok(true);
            }

            if answer == "n" || answer == "no" {
                return Ok(false);
            }

            if answer.is_empty() {
                return Ok(false);
            }
        }

        Ok(false)
    }
}

// -----------------------------------------------------------------------------------------------

pub struct FindKeyword<'a, T: Terminal> {
    message: String,
    content: &'a Buffer,
    screen: &'a mut Screen,
    status: &'a mut StatusBar,
    terminal: &'a mut T,
    current: &'a mut Cursor,
    source: Cursor,
}

impl<'a, T: Terminal> Prompt<T> for FindKeyword<'a, T> {
    fn message(&self) -> &str {
        self.message.as_str()
    }

    fn handle_event(&mut self, event: &Event, chars: &[char]) -> Result<KeyInput, Error> {
        let keyword = Row::from(chars);
        match &event {
            Event::Key(KeyEvent::End, _) => {
                self.current.move_to_xmax(self.content);
                Ok(KeyInput::Ok)
            }
            Event::Key(KeyEvent::PageUp, _) => {
                self.screen.move_up();
                self.current.move_up_screen(self.content, self.screen);
                Ok(KeyInput::Ok)
            }
            Event::Key(KeyEvent::PageDown, _) => {
                self.screen.move_down(self.content);
                self.current.move_down_screen(self.content, self.screen);
                Ok(KeyInput::Ok)
            }
            Event::Key(KeyEvent::Home, _) => {
                self.current.move_to_x0();
                Ok(KeyInput::Ok)
            }
            Event::Key(KeyEvent::ArrowLeft, _) => {
                self.current.move_left(self.content);
                Ok(KeyInput::Ok)
            }
            Event::Key(KeyEvent::ArrowUp, _) => {
                self.current.move_up(self.content);
                Ok(KeyInput::Ok)
            }
            Event::Key(KeyEvent::ArrowRight, _) => {
                self.current.move_right(self.content);
                Ok(KeyInput::Ok)
            }
            Event::Key(KeyEvent::ArrowDown, _) => {
                self.current.move_down(self.content);
                Ok(KeyInput::Ok)
            }
            Event::Key(KeyEvent::F3, KeyModifier::None) => {
                self.move_next_keyword(&keyword)?;
                Ok(KeyInput::Continue)
            }
            Event::Key(KeyEvent::F3, KeyModifier::Shift) => {
                self.move_previous_keyword(&keyword)?;
                Ok(KeyInput::Continue)
            }
            _ => Ok(KeyInput::Continue),
        }
    }

    fn handle_input_event(&mut self, chars: &[char]) -> Result<KeyInput, Error> {
        let keyword = Row::from(chars);
        if keyword.is_empty() {
            self.current.set(self.content, &(0, 0));
            self.clear_screen()?;
        } else {
            self.incremental_keyword(&keyword)?;
        }

        Ok(KeyInput::Continue)
    }

    fn screen(&self) -> &Screen {
        self.screen
    }

    fn terminal(&mut self) -> &mut T {
        self.terminal
    }
}

impl<'a, T: Terminal> FindKeyword<'a, T> {
    pub fn new(
        message: &str,
        cursor: &'a mut Cursor,
        content: &'a Buffer,
        screen: &'a mut Screen,
        status: &'a mut StatusBar,
        terminal: &'a mut T,
    ) -> Self {
        let source = cursor.clone();
        FindKeyword {
            message: message.to_string(),
            content,
            screen,
            status,
            terminal,
            current: cursor,
            source,
        }
    }

    pub fn current(&self) -> &Cursor {
        self.current
    }

    pub fn source(&self) -> &Cursor {
        &self.source
    }

    fn clear_screen(&mut self) -> Result<(), Error> {
        draw_screen(self.content, self.screen, self.terminal)?;
        draw_status(self.current, self.status, self.terminal)?;
        Ok(())
    }

    fn incremental_keyword(&mut self, keyword: &Row) -> Result<(), Error> {
        if let Some(at) = find_at(self.current, self.content, keyword) {
            self.mark_match(&at, keyword)?;
        } else {
            self.clear_screen()?;
        }
        Ok(())
    }

    fn mark_match<P: Coordinates>(&mut self, cursor: &P, keyword: &Row) -> Result<(), Error> {
        move_screen(self.current, cursor, self.content, self.screen, keyword);
        self.clear_screen()?;
        set_text_attribute(
            self.current,
            self.content,
            self.screen,
            self.terminal,
            keyword,
        )?;
        Ok(())
    }

    fn move_next_keyword(&mut self, keyword: &Row) -> Result<(), Error> {
        if let Some(at) = find_next_at(self.current, self.content, keyword) {
            self.mark_match(&at, keyword)?;
        }

        Ok(())
    }

    fn move_previous_keyword(&mut self, keyword: &Row) -> Result<(), Error> {
        if let Some(at) = rfind_next_at(self.current, self.content, keyword) {
            self.mark_match(&at, keyword)?;
        }

        Ok(())
    }
}

// -----------------------------------------------------------------------------------------------

pub struct Replace<'a, T: Terminal> {
    message: String,
    content: &'a mut Buffer,
    screen: &'a mut Screen,
    status: &'a mut StatusBar,
    terminal: &'a mut T,
    current: &'a mut Cursor,
    source: Cursor,
    keywords: Option<(Row, Row)>,
}

impl<'a, T: Terminal> Prompt<T> for Replace<'a, T> {
    fn callback_event(&mut self, _: &Event, chars: &mut Row) -> Result<(), Error> {
        if self.keywords.is_some() {
            chars.clear();
        }

        Ok(())
    }

    fn message(&self) -> &str {
        self.message.as_str()
    }

    fn handle_input_event(&mut self, chars: &[char]) -> Result<KeyInput, Error> {
        if let Some((source, replaced)) = self.keywords.clone() {
            match chars.iter().collect::<String>().as_str() {
                "y" => {
                    self.content
                        .replace(self.current, source.len(), replaced.column());
                }
                "n" => {}
                _ => return Ok(KeyInput::Continue),
            }

            if !self.move_next_keyword(&source)? {
                return Ok(KeyInput::Cancel);
            }
        }

        Ok(KeyInput::Continue)
    }

    fn screen(&self) -> &Screen {
        self.screen
    }

    fn terminal(&mut self) -> &mut T {
        self.terminal
    }
}

impl<'a, T: Terminal> Replace<'a, T> {
    pub fn new(
        message: &str,
        cursor: &'a mut Cursor,
        content: &'a mut Buffer,
        screen: &'a mut Screen,
        status: &'a mut StatusBar,
        terminal: &'a mut T,
    ) -> Self {
        let source = cursor.clone();
        Replace {
            message: message.to_string(),
            content,
            screen,
            status,
            terminal,
            current: cursor,
            source,
            keywords: None,
        }
    }

    pub fn replace(&mut self, value: Option<&str>) -> Result<(), Error> {
        let mut esc_at = self.source.clone();

        if let Some(source) = self.input(value)? {
            self.message = format!("{} {} -> ", &self.message, &source.to_string_at(0));
            if let Some(replaced) = self.input(None)? {
                self.keywords = Some((source.clone(), replaced.clone()));

                if self.move_keyword_at_current(&source)? {
                    self.message =
                        format!("{}{} (y/n): ", &self.message, &replaced.to_string_at(0));
                    while self.handle_events(None)?.is_some() {}

                    esc_at = self.current.clone();
                }
            }
        }

        self.current.set(self.content, &esc_at);
        Ok(())
    }

    fn clear_screen(&mut self) -> Result<(), Error> {
        draw_screen(self.content, self.screen, self.terminal)?;
        draw_status(self.current, self.status, self.terminal)?;
        Ok(())
    }

    fn input(&mut self, value: Option<&str>) -> Result<Option<Row>, Error> {
        while let Some(value) = self.handle_events(value)? {
            if value.is_empty() {
                continue;
            }

            let row = Row::from(value);
            return Ok(Some(row));
        }

        Ok(None)
    }

    fn mark_match<P: Coordinates>(&mut self, cursor: &P, keyword: &Row) -> Result<(), Error> {
        move_screen(self.current, cursor, self.content, self.screen, keyword);
        self.clear_screen()?;
        set_text_attribute(
            self.current,
            self.content,
            self.screen,
            self.terminal,
            keyword,
        )?;
        Ok(())
    }

    fn move_first_keyword(&mut self, keyword: &Row) -> Result<bool, Error> {
        if let Some(at) = find_at(&Cursor::default(), self.content, keyword) {
            self.mark_match(&at, keyword)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn move_keyword_at_current(&mut self, keyword: &Row) -> Result<bool, Error> {
        if let Some(at) = find_at(self.current, self.content, keyword) {
            self.mark_match(&at, keyword)?;
            Ok(true)
        } else {
            self.move_first_keyword(keyword)
        }
    }

    fn move_next_keyword(&mut self, keyword: &Row) -> Result<bool, Error> {
        if let Some(at) = find_next_at(self.current, self.content, keyword) {
            self.mark_match(&at, keyword)?;
            Ok(true)
        } else {
            self.move_first_keyword(keyword)
        }
    }
}

// -----------------------------------------------------------------------------------------------

fn draw_screen<T: Terminal>(
    content: &Buffer,
    screen: &mut Screen,
    terminal: &mut T,
) -> Result<(), Error> {
    // Delete text decoration.
    screen.force_update();

    screen.draw(content, &Select::default(), terminal)?;
    Ok(())
}

fn draw_status<T: Terminal>(
    cursor: &Cursor,
    status: &mut StatusBar,
    terminal: &mut T,
) -> Result<(), Error> {
    status.set_cursor(cursor);
    status.draw(terminal)?;
    Ok(())
}

fn find_at(cursor: &Cursor, content: &Buffer, keyword: &Row) -> Option<(usize, usize)> {
    content.find_at(cursor, &keyword.to_string_at(0))
}

fn find_next_at(cursor: &Cursor, content: &Buffer, keyword: &Row) -> Option<(usize, usize)> {
    let mut c = cursor.clone();
    c.move_right(content);

    if let Some((x, y)) = content.find_at(&c, &keyword.to_string_at(0)) {
        Some((x, y))
    } else {
        None
    }
}

fn move_screen<P: Coordinates>(
    cursor: &mut Cursor,
    at: &P,
    content: &Buffer,
    screen: &mut Screen,
    keyword: &Row,
) {
    cursor.set(content, at);

    let keyword_width = keyword.width();
    if keyword_width < screen.width() {
        let mut last_ch = cursor.clone();
        last_ch.set_x(content, cursor.x() + keyword.len() - 1);
        screen.fit(content, &last_ch.render(content));
    }

    screen.fit(content, &cursor.render(content));
}

fn rfind_next_at(cursor: &Cursor, content: &Buffer, keyword: &Row) -> Option<(usize, usize)> {
    let mut c = cursor.clone();
    c.move_left(content);

    if let Some((x, y)) = content.rfind_at(&c, &keyword.to_string_at(0)) {
        Some((x, y))
    } else {
        None
    }
}

fn set_text_attribute<T: Terminal>(
    cursor: &Cursor,
    content: &Buffer,
    screen: &Screen,
    terminal: &mut T,
    keyword: &Row,
) -> Result<(), Error> {
    let render = cursor.render(content);
    let keyword_width = keyword.width();
    let length = min(keyword_width, screen.right() - render.x() + 1);
    terminal.set_text_attribute(
        render.x() - screen.left(),
        render.y() - screen.top(),
        length,
    )?;
    Ok(())
}
