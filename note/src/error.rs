#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Utf16(std::char::DecodeUtf16Error),
    Win32(windows::core::Error),
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error::Io(error)
    }
}

impl From<std::char::DecodeUtf16Error> for Error {
    fn from(error: std::char::DecodeUtf16Error) -> Self {
        Error::Utf16(error)
    }
}

impl From<windows::core::Error> for Error {
    fn from(error: windows::core::Error) -> Self {
        Error::Win32(error)
    }
}
