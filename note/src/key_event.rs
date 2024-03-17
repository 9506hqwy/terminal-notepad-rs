#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    Key(KeyEvent, KeyModifier),
    Window(WindowEvent),
}

impl From<(KeyEvent, KeyModifier)> for Event {
    fn from(value: (KeyEvent, KeyModifier)) -> Self {
        Event::Key(value.0, value.1)
    }
}

impl From<WindowEvent> for Event {
    fn from(value: WindowEvent) -> Self {
        Event::Window(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum KeyEvent {
    // virtual key codes
    BackSpace,
    Enter,
    Escape,
    End,
    PageUp,
    PageDown,
    Home,
    ArrowLeft,
    ArrowUp,
    ArrowRight,
    ArrowDown,
    Delete,
    F3,
    // ctrl modifier
    Copy,
    DeleteRow,
    Find,
    Exit,
    Paste,
    Save,
    Undo,
    // other
    Char(char),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum KeyModifier {
    None,
    Shift,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WindowEvent {
    Resize,
}
