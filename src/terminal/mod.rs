//! Terminal detection and capabilities

use crossterm::cursor;
use crossterm::terminal;
use crossterm::ExecutableCommand;
use std::io;

/// Get the current terminal size in (columns, rows)
pub fn get_terminal_size() -> io::Result<(usize, usize)> {
    let (cols, rows) = terminal::size()?;
    Ok((cols as usize, rows as usize))
}

/// Clear the terminal screen
pub fn clear_screen() -> io::Result<()> {
    let mut stdout = io::stdout();
    stdout.execute(terminal::Clear(terminal::ClearType::All))?;
    Ok(())
}

/// Check if terminal supports 24-bit truecolor
///
/// This is a best-effort check. Modern terminals (kitty, iTerm2, VS Code terminal,
/// Windows Terminal, etc.) support truecolor.
pub fn supports_truecolor() -> bool {
    // Check environment variables that indicate truecolor support
    std::env::var("COLORTERM")
        .map(|v| v.contains("truecolor") || v.contains("24bit"))
        .unwrap_or(false)
        || std::env::var("TERM_PROGRAM")
            .map(|v| matches!(v.as_str(), "iTerm.app" | "vscode" | "WezTerm"))
            .unwrap_or(false)
        || std::env::var("WT_SESSION").is_ok() // Windows Terminal
        || std::env::var("KITTY_WINDOW_ID").is_ok() // Kitty
}

/// Check if terminal supports 256-color palette
pub fn supports_256_color() -> bool {
    std::env::var("TERM")
        .map(|term| term.contains("256color") || term.contains("xterm-256color") || term.contains("screen-256color"))
        .unwrap_or(false)
}

/// Character aspect ratio for terminal fonts
///
/// Most monospace terminal fonts have an aspect ratio around 0.5 (width:height).
/// This is important for calculating proper image dimensions.
pub const CHAR_ASPECT_RATIO: f64 = 0.5;

/// Hide the terminal cursor
pub fn hide_cursor() -> io::Result<()> {
    let mut stdout = io::stdout();
    stdout.execute(cursor::Hide)?;
    Ok(())
}

/// Show the terminal cursor
pub fn show_cursor() -> io::Result<()> {
    let mut stdout = io::stdout();
    stdout.execute(cursor::Show)?;
    Ok(())
}

/// Move cursor to top-left corner (0, 0)
pub fn move_cursor_home() -> io::Result<()> {
    let mut stdout = io::stdout();
    stdout.execute(cursor::MoveTo(0, 0))?;
    Ok(())
}
