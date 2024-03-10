use note::editor::Editor;
use note::error::Error;
use note::terminal::{Terminal, WindowsCon};
use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Error> {
    let filename = env::args().nth(1).map(PathBuf::from);

    let mut terminal = WindowsCon {};
    terminal.alternate_screen_buffer()?;
    terminal.enable_raw_mode()?;

    let mut editor = Editor::new(filename.as_deref(), terminal)?;

    editor.init()?;

    loop {
        editor.handle_events()?;
        editor.refresh()?;
    }
}
