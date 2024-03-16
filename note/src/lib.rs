pub mod buffer;
pub mod cursor;
pub mod editor;
pub mod error;
pub mod history;
pub mod key_event;
pub mod prompt;
pub mod screen;
pub mod terminal;

mod windows;

// https://learn.microsoft.com/en-us/windows/console/char-info-str
#[derive(Clone, Copy, Debug)]
pub enum Color {
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Yellow = 6,
    White = 7,
}
