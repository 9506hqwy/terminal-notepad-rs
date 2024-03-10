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
