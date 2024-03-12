use crate::buffer::{Buffer, Row};
use crate::cursor::{Coordinates, Cursor};
use crate::error::Error;
use crate::key_event::{KeyEvent, KeyModifier};
use crate::screen::{MessageBar, Screen, StatusBar};
use crate::terminal::Terminal;

pub trait Prompt<T: Terminal> {
    #[allow(unused_variables)]
    fn callback_event(
        &mut self,
        key: &(KeyEvent, KeyModifier),
        chars: &[char],
    ) -> Result<(), Error> {
        Ok(())
    }

    fn handle_events(&mut self) -> Result<Option<String>, Error> {
        let prompt = MessageBar::new(self.screen(), self.message());

        prompt.draw(self.terminal())?;
        let (prompt_x, prompt_y) = self.terminal().get_cursor_position()?;
        let mut key = self.read_key_timeout()?;

        let mut chars = Row::default();
        while match key {
            (KeyEvent::BackSpace, _) => {
                chars.remove(chars.len() - 1);
                self.handle_input_event(chars.column())?
            }
            (KeyEvent::Enter, _) => false,
            (KeyEvent::Escape, _) => return Ok(None),
            (KeyEvent::Char(ch), _) if !ch.is_ascii_control() => {
                chars.insert(chars.len(), ch);
                self.handle_input_event(chars.column())?
            }
            _ => self.handle_event(&key, chars.column())?,
        } {
            self.callback_event(&key, chars.column())?;

            prompt.draw(self.terminal())?;
            chars.truncate_width(self.screen().width() - prompt_x - 1);
            self.terminal()
                .write(prompt_x, prompt_y, chars.column(), false)?;
            key = self.read_key_timeout()?;
        }

        Ok(Some(chars.to_string_at(0)))
    }

    #[allow(unused_variables)]
    fn handle_event(
        &mut self,
        key: &(KeyEvent, KeyModifier),
        chars: &[char],
    ) -> Result<bool, Error> {
        Ok(true)
    }

    #[allow(unused_variables)]
    fn handle_input_event(&mut self, chars: &[char]) -> Result<bool, Error> {
        Ok(true)
    }

    fn message(&self) -> &str;

    fn read_key_timeout(&self) -> Result<(KeyEvent, KeyModifier), Error> {
        T::read_key_timeout()
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

    fn handle_event(
        &mut self,
        key: &(KeyEvent, KeyModifier),
        chars: &[char],
    ) -> Result<bool, Error> {
        let keyword = chars.iter().collect::<String>();
        match &key {
            (KeyEvent::F3, KeyModifier::None) => {
                self.move_next_keyword(&keyword)?;
                Ok(true)
            }
            (KeyEvent::F3, KeyModifier::Shift) => {
                self.move_previous_keyword(&keyword)?;
                Ok(true)
            }
            _ => Ok(true),
        }
    }

    fn handle_input_event(&mut self, chars: &[char]) -> Result<bool, Error> {
        let keyword = chars.iter().collect::<String>();
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
        // FIXEDME: screen move belong on keyword width.
        let render = self.current.render(self.content);
        self.screen.fit(self.content, &render);
        self.screen.clear(self.terminal)?;
        self.screen.draw(self.content, self.terminal)?;
        self.status.draw(self.current, self.terminal)?;
        Ok(())
    }

    fn incremental_keyword(&mut self, keyword: &str) -> Result<(), Error> {
        if let Some((x, y)) = self.content.find_at(self.current, keyword) {
            self.current.set(self.content, &(x, y));
            self.clear_screen()?;
            self.set_text_attribute(keyword)?;
        } else {
            self.clear_screen()?;
        }
        Ok(())
    }

    fn move_next_keyword(&mut self, keyword: &str) -> Result<(), Error> {
        let mut c = self.current.clone();
        c.move_right(self.content);

        if let Some((x, y)) = self.content.find_at(&c, keyword) {
            self.current.set(self.content, &(x, y));
            self.clear_screen()?;
            self.set_text_attribute(keyword)?;
        }

        Ok(())
    }

    fn move_previous_keyword(&mut self, keyword: &str) -> Result<(), Error> {
        let mut c = self.current.clone();
        c.move_left(self.content);

        if let Some((x, y)) = self.content.rfind_at(&c, keyword) {
            self.current.set(self.content, &(x, y));
            self.clear_screen()?;
            self.set_text_attribute(keyword)?;
        }

        Ok(())
    }

    fn set_text_attribute(&mut self, keyword: &str) -> Result<(), Error> {
        let render = self.current.render(self.content);
        let length = Row::from(keyword.chars().collect::<Vec<char>>()).width();
        self.terminal.set_text_attribute(
            render.x() - self.screen.left(),
            render.y() - self.screen.top(),
            length,
        )?;
        Ok(())
    }
}
