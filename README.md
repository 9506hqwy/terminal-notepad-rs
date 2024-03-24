# Terminal Notepad for Windows

This crate provides a minimum text editor in windows command prompt.

## Features

- Text encoding is UTF-8 only.
- New line code is CRLF only.
- Incremental text search.
- Undo.
- Select text area for copy or cut (Shift+Arrow).

## Keyboard Shortcut

| Key    | Operation                      |
| ------ | ------------------------------ |
| Ctrl+A | Move cursor to start of line   |
| Ctrl+C | Copy text in selected area     |
| Ctrl+E | Move cursor to end of line     |
| Ctrl+F | Find text keyword              |
| Ctrl+G | Go to line                     |
| Ctrl+H | Replace text                   |
| Ctrl+K | Cut text up to end of line     |
| Ctrl+N | Move down cursor to below line |
| Ctrl+P | Move up cursor to above line   |
| Ctrl+Q | Close editor                   |
| Ctrl+S | Save to file                   |
| Ctrl+V | Paste text after copy or cut   |
| Ctrl+X | Cut text in selected area      |
| Ctrl+Z | Undo                           |
