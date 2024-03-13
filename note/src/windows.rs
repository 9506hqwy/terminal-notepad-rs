use crate::error::Error;
use crate::key_event::{KeyEvent, KeyModifier};
use windows::Win32::Foundation::{GENERIC_READ, GENERIC_WRITE, HANDLE};
use windows::Win32::Storage::FileSystem::{FILE_SHARE_READ, FILE_SHARE_WRITE};
use windows::Win32::System::Console::{
    CreateConsoleScreenBuffer, FillConsoleOutputAttribute, FillConsoleOutputCharacterA,
    GetConsoleMode, GetConsoleScreenBufferInfo, GetStdHandle, ReadConsoleInputW,
    ScrollConsoleScreenBufferA, SetConsoleActiveScreenBuffer, SetConsoleCursorPosition,
    SetConsoleMode, SetConsoleOutputCP, SetConsoleScreenBufferSize, SetConsoleTextAttribute,
    SetStdHandle, WriteConsoleA, WriteConsoleOutputW, CHAR_INFO, CHAR_INFO_0,
    COMMON_LVB_LEADING_BYTE, COMMON_LVB_REVERSE_VIDEO, COMMON_LVB_TRAILING_BYTE,
    CONSOLE_CHARACTER_ATTRIBUTES, CONSOLE_MODE, CONSOLE_SCREEN_BUFFER_INFO,
    CONSOLE_TEXTMODE_BUFFER, COORD, ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT, ENABLE_PROCESSED_INPUT,
    ENABLE_PROCESSED_OUTPUT, ENABLE_WRAP_AT_EOL_OUTPUT, INPUT_RECORD, KEY_EVENT, SHIFT_PRESSED,
    SMALL_RECT, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
};

pub fn alternate_screen_buffer() -> Result<HANDLE, Error> {
    // https://learn.microsoft.com/en-us/windows/console/createconsolescreenbuffer
    let handle = unsafe {
        CreateConsoleScreenBuffer(
            GENERIC_READ.0 | GENERIC_WRITE.0,
            FILE_SHARE_READ.0 | FILE_SHARE_WRITE.0,
            None,
            CONSOLE_TEXTMODE_BUFFER,
            None,
        )
    }?;
    let info = get_stdout_buffer_info()?;
    // https://learn.microsoft.com/en-us/windows/console/setconsolescreenbuffersize
    unsafe { SetConsoleScreenBufferSize(handle, info.dwSize) }?;
    // https://learn.microsoft.com/en-us/windows/console/setconsoleactivescreenbuffer
    unsafe { SetConsoleActiveScreenBuffer(handle) }?;
    // https://learn.microsoft.com/en-us/windows/console/setstdhandle
    unsafe { SetStdHandle(STD_OUTPUT_HANDLE, handle) }?;
    // https://learn.microsoft.com/en-us/windows/console/setconsoleoutputcp
    unsafe { SetConsoleOutputCP(65001) }?;
    Ok(handle)
}

pub fn clear_screen() -> Result<(), Error> {
    // https://learn.microsoft.com/en-us/windows/console/clearing-the-screen
    scroll_up_whole_buffer()?;
    set_cursor_position(0, 0)?;
    Ok(())
}

pub fn enable_raw_mode() -> Result<(), Error> {
    // https://learn.microsoft.com/en-us/windows/console/high-level-console-modes
    {
        // https://learn.microsoft.com/en-us/windows/console/getconsolemode
        let mut mode = CONSOLE_MODE::default();
        unsafe { GetConsoleMode(stdin()?, &mut mode) }?;

        // https://learn.microsoft.com/en-us/windows/console/setconsolemode
        mode &= !(ENABLE_ECHO_INPUT | ENABLE_LINE_INPUT | ENABLE_PROCESSED_INPUT);
        unsafe { SetConsoleMode(stdin()?, mode) }?;
    }

    {
        let mut mode = CONSOLE_MODE::default();
        unsafe { GetConsoleMode(stdout()?, &mut mode) }?;

        mode &= !(ENABLE_WRAP_AT_EOL_OUTPUT | ENABLE_PROCESSED_OUTPUT);
        unsafe { SetConsoleMode(stdout()?, mode) }?;
    }

    Ok(())
}

pub fn get_cursor_position() -> Result<(usize, usize), Error> {
    let info = get_stdout_buffer_info()?;
    Ok((
        info.dwCursorPosition.X as usize,
        info.dwCursorPosition.Y as usize,
    ))
}

pub fn get_screen_size() -> Result<(usize, usize), Error> {
    // FIXEDME: in case windows terminal, screen size is incorrect after resizing window.
    let info = get_stdout_buffer_info()?;
    Ok((
        info.srWindow.Right as usize + 1,
        info.srWindow.Bottom as usize + 1,
    ))
}

pub fn read_key() -> Result<(KeyEvent, KeyModifier), Error> {
    loop {
        let mut buf = [INPUT_RECORD::default(); 1];
        let mut num = 1u32;
        unsafe { ReadConsoleInputW(stdin()?, buf.as_mut_slice(), &mut num) }?;

        if buf[0].EventType != (KEY_EVENT as u16) {
            continue;
        }

        if !unsafe { buf[0].Event.KeyEvent.bKeyDown }.as_bool() {
            continue;
        }

        // https://learn.microsoft.com/en-us/windows/console/key-event-record-str
        let state = unsafe { buf[0].Event.KeyEvent.dwControlKeyState };
        let modifier = match state {
            SHIFT_PRESSED => KeyModifier::Shift,
            _ => KeyModifier::None,
        };

        // https://learn.microsoft.com/en-us/windows/win32/inputdev/virtual-key-codes
        let v_key = unsafe { buf[0].Event.KeyEvent.wVirtualKeyCode };
        match v_key {
            0x08 => return Ok((KeyEvent::BackSpace, modifier)),
            0x0D => return Ok((KeyEvent::Enter, modifier)),
            0x1B => return Ok((KeyEvent::Escape, modifier)),
            0x23 => return Ok((KeyEvent::End, modifier)),
            0x21 => return Ok((KeyEvent::PageUp, modifier)),
            0x22 => return Ok((KeyEvent::PageDown, modifier)),
            0x24 => return Ok((KeyEvent::Home, modifier)),
            0x25 => return Ok((KeyEvent::ArrowLeft, modifier)),
            0x26 => return Ok((KeyEvent::ArrowUp, modifier)),
            0x27 => return Ok((KeyEvent::ArrowRight, modifier)),
            0x28 => return Ok((KeyEvent::ArrowDown, modifier)),
            0x2E => return Ok((KeyEvent::Delete, modifier)),
            0x72 => return Ok((KeyEvent::F3, modifier)),
            _ => {}
        }

        let code = unsafe { buf[0].Event.KeyEvent.uChar.UnicodeChar };
        if let Some(ch) = char::decode_utf16([code]).next() {
            let ch = ch?;
            if ch.is_ascii_control() {
                // https://doc.rust-lang.org/std/ascii/enum.Char.html
                match ch as u8 {
                    6 => return Ok((KeyEvent::Find, modifier)),  // Ctrl+'F'
                    17 => return Ok((KeyEvent::Exit, modifier)), // Ctrl+'Q'
                    19 => return Ok((KeyEvent::Save, modifier)), // Ctrl+'S'
                    _ => {}
                }
            }

            return Ok((KeyEvent::Char(ch), modifier));
        }
    }
}

pub fn set_cursor_position(x: usize, y: usize) -> Result<(), Error> {
    // https://learn.microsoft.com/en-us/windows/console/setconsolecursorposition
    let pos = COORD {
        X: x as i16,
        Y: y as i16,
    };
    unsafe { SetConsoleCursorPosition(stdout()?, pos) }?;
    Ok(())
}

pub fn set_text_attribute(x: usize, y: usize, length: usize) -> Result<(), Error> {
    // https://learn.microsoft.com/en-us/windows/console/fillconsoleoutputattribute
    let info = get_stdout_buffer_info()?;
    let attr = info.wAttributes | COMMON_LVB_REVERSE_VIDEO;
    let at = COORD {
        X: x as i16,
        Y: y as i16,
    };
    let mut written = 0;
    unsafe { FillConsoleOutputAttribute(stdout()?, attr.0, length as u32, at, &mut written) }?;
    Ok(())
}

pub fn write_console(x: usize, y: usize, row: &[char], rev: bool) -> Result<(), Error> {
    let info = get_stdout_buffer_info()?;

    // https://learn.microsoft.com/en-us/windows/console/setconsoletextattribute
    let attr = info.wAttributes
        | if rev {
            COMMON_LVB_REVERSE_VIDEO
        } else {
            CONSOLE_CHARACTER_ATTRIBUTES(0)
        };

    if rev {
        unsafe { SetConsoleTextAttribute(stdout()?, attr) }?;
    }

    // https://learn.microsoft.com/en-us/windows/console/fillconsoleoutputcharacter
    let width = (info.srWindow.Right as u32) - x as u32;
    let spece_at = COORD {
        X: x as i16,
        Y: y as i16,
    };
    let mut written = 0;
    unsafe { FillConsoleOutputCharacterA(stdout()?, b' ' as i8, width, spece_at, &mut written) }?;

    set_cursor_position(x, y)?;

    let buffer = row.iter().collect::<String>();
    unsafe { WriteConsoleA(stdout()?, buffer.as_bytes(), None, None) }?;

    if rev {
        unsafe { SetConsoleTextAttribute(stdout()?, info.wAttributes) }?;
    }

    Ok(())
}

// -----------------------------------------------------------------------------------------------

#[allow(dead_code)]
fn control_key(c: u8) -> u8 {
    // https://www.asciitable.com/
    // e.g.
    // CTRL+Q: b'q' & 0x1F = 17
    c & 0x1F
}

fn get_stdout_buffer_info() -> Result<CONSOLE_SCREEN_BUFFER_INFO, Error> {
    // https://learn.microsoft.com/en-us/windows/console/getconsolescreenbufferinfo
    let mut info = CONSOLE_SCREEN_BUFFER_INFO::default();
    unsafe { GetConsoleScreenBufferInfo(stdout()?, &mut info) }?;
    Ok(info)
}

fn scroll_up_whole_buffer() -> Result<(), Error> {
    // https://learn.microsoft.com/en-us/windows/console/scrollconsolescreenbuffer
    let info = get_stdout_buffer_info()?;
    let rect = SMALL_RECT {
        Right: info.dwSize.X,
        Bottom: info.dwSize.Y,
        ..Default::default()
    };
    let origin = COORD {
        X: 0,
        Y: 0 - info.dwSize.Y,
    };
    let fill = CHAR_INFO {
        Attributes: info.wAttributes.0,
        Char: CHAR_INFO_0 {
            AsciiChar: b' ' as i8,
        },
    };
    unsafe { ScrollConsoleScreenBufferA(stdout()?, &rect, None, origin, &fill) }?;
    Ok(())
}

fn stdin() -> Result<HANDLE, Error> {
    // https://learn.microsoft.com/en-us/windows/console/getstdhandle
    let handle = unsafe { GetStdHandle(STD_INPUT_HANDLE) }?;
    Ok(handle)
}

fn stdout() -> Result<HANDLE, Error> {
    // https://learn.microsoft.com/en-us/windows/console/getstdhandle
    let handle = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) }?;
    Ok(handle)
}

#[allow(dead_code)]
fn write_console_legacy(_x: usize, y: usize, row: &[char], rev: bool) -> Result<(), Error> {
    let info = get_stdout_buffer_info()?;

    let attr = info.wAttributes.0 | if rev { COMMON_LVB_REVERSE_VIDEO.0 } else { 0 };

    let mut chars = vec![];
    for ch in row {
        let mut buf = [0; 2];
        ch.encode_utf16(&mut buf);
        if ch.is_ascii() {
            chars.push(CHAR_INFO {
                Attributes: attr,
                Char: CHAR_INFO_0 {
                    UnicodeChar: buf[0],
                },
            });
        } else {
            chars.push(CHAR_INFO {
                Attributes: attr | COMMON_LVB_LEADING_BYTE.0,
                Char: CHAR_INFO_0 {
                    UnicodeChar: buf[0],
                },
            });
            chars.push(CHAR_INFO {
                Attributes: attr | COMMON_LVB_TRAILING_BYTE.0,
                Char: CHAR_INFO_0 {
                    UnicodeChar: buf[1],
                },
            });
        }
    }

    let buffer_size = COORD {
        X: chars.len() as i16,
        Y: 1,
    };

    let buffer_at = COORD::default();

    let mut region = SMALL_RECT {
        Left: 0,
        Top: y as i16,
        Right: chars.len() as i16 - 1,
        Bottom: y as i16,
    };

    unsafe {
        // `CHAR_INFO` is single utf16, so `WriteConsoleOutput`` can not write surrogate pair.
        WriteConsoleOutputW(
            stdout()?,
            chars.as_ptr(),
            buffer_size,
            buffer_at,
            &mut region,
        )
    }?;

    Ok(())
}
