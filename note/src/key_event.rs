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
    DeleteRow,
    Find,
    Exit,
    Save,
    // other
    Char(char),
}

pub enum KeyModifier {
    None,
    Shift,
}

pub enum WindowEvent {
    Resize,
}
