use crate::buffer::{Buffer, Row};
use crate::cursor::{Coordinates, Cursor};
use crate::editor::Select;
use crate::error::Error;
use crate::key_event::{Event, KeyEvent, KeyModifier};
use crate::screen::{MessageBar, Screen, StatusBar};
use crate::terminal::Terminal;
use crate::Color;
use std::cmp::min;

pub trait Prompt<T: Terminal> {
    #[allow(unused_variables)]
    fn callback_event(&mut self, event: &Event, chars: &[char]) -> Result<(), Error> {
        Ok(())
    }

    fn handle_events(&mut self) -> Result<Option<String>, Error> {
        let mut prompt = MessageBar::new(self.screen(), self.message());
        prompt.set_fg_color(Color::Cyan);

        prompt.draw(self.terminal())?;
        let (prompt_x, prompt_y) = self.terminal().get_cursor_position()?;
        let mut event = self.read_event_timeout()?;

        let mut chars = Row::default();
        while match event {
            Event::Key(KeyEvent::BackSpace, _) => {
                if !chars.is_empty() {
                    chars.remove(chars.len() - 1);
                    self.handle_input_event(chars.column())?
                } else {
                    true
                }
            }
            Event::Key(KeyEvent::Enter, _) => false,
            Event::Key(KeyEvent::Escape, _) => return Ok(None),
            Event::Key(KeyEvent::Char(ch), _) if !ch.is_ascii_control() => {
                chars.insert(chars.len(), ch);
                self.handle_input_event(chars.column())?
            }
            Event::Key(..) => self.handle_event(&event, chars.column())?,
            // TODO: resize screen
            _ => true,
        } {
            self.callback_event(&event, chars.column())?;

            prompt.draw(self.terminal())?;
            chars.truncate_width(self.screen().width() - prompt_x - 1);
            self.terminal()
                .write(prompt_x, prompt_y, chars.column(), Color::White, false)?;
            event = self.read_event_timeout()?;
        }

        Ok(Some(chars.to_string_at(0)))
    }

    #[allow(unused_variables)]
    fn handle_event(&mut self, event: &Event, chars: &[char]) -> Result<bool, Error> {
        Ok(true)
    }

    #[allow(unused_variables)]
    fn handle_input_event(&mut self, chars: &[char]) -> Result<bool, Error> {
        Ok(true)
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
        loop {
            if let Some(yes_no) = self.handle_events()? {
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
        }
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

    fn handle_event(&mut self, event: &Event, chars: &[char]) -> Result<bool, Error> {
        let keyword = Row::from(chars);
        match &event {
            Event::Key(KeyEvent::End, _) => {
                self.current.move_to_xmax(self.content);
                Ok(false)
            }
            Event::Key(KeyEvent::PageUp, _) => {
                self.screen.move_up();
                self.current.move_up_screen(self.content, self.screen);
                Ok(false)
            }
            Event::Key(KeyEvent::PageDown, _) => {
                self.screen.move_down(self.content);
                self.current.move_down_screen(self.content, self.screen);
                Ok(false)
            }
            Event::Key(KeyEvent::Home, _) => {
                self.current.move_to_x0();
                Ok(false)
            }
            Event::Key(KeyEvent::ArrowLeft, _) => {
                self.current.move_left(self.content);
                Ok(false)
            }
            Event::Key(KeyEvent::ArrowUp, _) => {
                self.current.move_up(self.content);
                Ok(false)
            }
            Event::Key(KeyEvent::ArrowRight, _) => {
                self.current.move_right(self.content);
                Ok(false)
            }
            Event::Key(KeyEvent::ArrowDown, _) => {
                self.current.move_down(self.content);
                Ok(false)
            }
            Event::Key(KeyEvent::F3, KeyModifier::None) => {
                self.move_next_keyword(&keyword)?;
                Ok(true)
            }
            Event::Key(KeyEvent::F3, KeyModifier::Shift) => {
                self.move_previous_keyword(&keyword)?;
                Ok(true)
            }
            _ => Ok(true),
        }
    }

    fn handle_input_event(&mut self, chars: &[char]) -> Result<bool, Error> {
        let keyword = Row::from(chars);
        if keyword.is_empty() {
            self.current.set(self.content, &(0, 0));
            self.clear_screen()?;
        } else {
            self.incremental_keyword(&keyword)?;
        }

        Ok(true)
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
        // Delete text decoration.
        self.screen.force_update();

        self.screen
            .draw(self.content, &Select::default(), self.terminal)?;
        self.status.set_cursor(self.current);
        self.status.draw(self.terminal)?;
        Ok(())
    }

    fn incremental_keyword(&mut self, keyword: &Row) -> Result<(), Error> {
        if let Some((x, y)) = self.content.find_at(self.current, &keyword.to_string_at(0)) {
            self.mark_match(&(x, y), keyword)?;
        } else {
            self.clear_screen()?;
        }
        Ok(())
    }

    fn mark_match<P: Coordinates>(&mut self, cursor: &P, keyword: &Row) -> Result<(), Error> {
        self.current.set(self.content, cursor);

        let keyword_width = keyword.width();
        if keyword_width < self.screen().width() {
            let mut last_ch = self.current.clone();
            last_ch.set_x(self.content, self.current.x() + keyword.len() - 1);
            self.screen.fit(self.content, &last_ch.render(self.content));
        }

        self.screen
            .fit(self.content, &self.current.render(self.content));

        self.clear_screen()?;
        self.set_text_attribute(keyword)?;
        Ok(())
    }

    fn move_next_keyword(&mut self, keyword: &Row) -> Result<(), Error> {
        let mut c = self.current.clone();
        c.move_right(self.content);

        if let Some((x, y)) = self.content.find_at(&c, &keyword.to_string_at(0)) {
            self.mark_match(&(x, y), keyword)?;
        }

        Ok(())
    }

    fn move_previous_keyword(&mut self, keyword: &Row) -> Result<(), Error> {
        let mut c = self.current.clone();
        c.move_left(self.content);

        if let Some((x, y)) = self.content.rfind_at(&c, &keyword.to_string_at(0)) {
            self.mark_match(&(x, y), keyword)?;
        }

        Ok(())
    }

    fn set_text_attribute(&mut self, keyword: &Row) -> Result<(), Error> {
        let render = self.current.render(self.content);
        let keyword_width = keyword.width();
        let length = min(keyword_width, self.screen.right() - render.x() + 1);
        self.terminal.set_text_attribute(
            render.x() - self.screen.left(),
            render.y() - self.screen.top(),
            length,
        )?;
        Ok(())
    }
}
