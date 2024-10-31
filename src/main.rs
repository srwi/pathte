use clipboard_win::{formats, get_clipboard, is_format_avail, set_clipboard, SysResult};
use std::path::Path;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    keybd_event, RegisterHotKey, KEYBD_EVENT_FLAGS, MOD_ALT, MOD_CONTROL, MOD_WIN, VK_CONTROL,
    VK_E, VK_V,
};
use windows::Win32::UI::WindowsAndMessaging::{DispatchMessageW, GetMessageW, MSG, WM_HOTKEY};

#[derive(Debug)]
enum ClipboardError {
    NoUnicodeText,
    ClipboardError(String),
}

impl std::fmt::Display for ClipboardError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ClipboardError::NoUnicodeText => write!(f, "Unicode text not available in clipboard"),
            ClipboardError::ClipboardError(e) => write!(f, "Clipboard error: {}", e),
        }
    }
}

fn main() -> windows::core::Result<()> {
    println!("Path converter started. Press Ctrl+Win+V to convert and paste paths.");
    println!("The application is running in the background...");

    let result = unsafe { RegisterHotKey(HWND(0), 1, MOD_ALT | MOD_WIN, VK_E.0 as u32) };

    if !result.as_bool() {
        println!("Failed to register hotkey. Is another instance running?");
        return Ok(());
    }

    let mut msg = MSG::default();
    unsafe {
        while GetMessageW(&mut msg, HWND(0), 0, 0).as_bool() {
            if msg.message == WM_HOTKEY {
                if let Err(e) = handle_hotkey() {
                    eprintln!("Error: {}", e);
                }
            }
            DispatchMessageW(&msg);
        }
    }

    Ok(())
}

fn handle_hotkey() -> Result<(), ClipboardError> {
    match get_clipboard_text() {
        Ok(text) => {
            if let Some(converted) = convert_if_valid_path(&text) {
                set_clipboard_text(&converted)
                    .map_err(|e| ClipboardError::ClipboardError(e.to_string()))?;
                simulate_paste();
            } else {
                simulate_paste();
            }
            Ok(())
        }
        Err(e) => Err(e),
    }
}

fn convert_if_valid_path(text: &str) -> Option<String> {
    if !text.contains('\\') {
        return None;
    }

    let path = Path::new(text);

    if path.components().count() <= 1 {
        return None;
    }

    Some(text.replace('\\', "/"))
}

fn get_clipboard_text() -> Result<String, ClipboardError> {
    if !is_format_avail(formats::CF_UNICODETEXT) {
        return Err(ClipboardError::NoUnicodeText);
    }

    get_clipboard(formats::Unicode).map_err(|e| ClipboardError::ClipboardError(e.to_string()))
}

fn set_clipboard_text(text: &str) -> SysResult<()> {
    set_clipboard(formats::Unicode, text)
}

fn simulate_paste() {
    unsafe {
        keybd_event(VK_CONTROL.0 as u8, 0, KEYBD_EVENT_FLAGS(0), 0);
        keybd_event(VK_V.0 as u8, 0, KEYBD_EVENT_FLAGS(0), 0);

        keybd_event(
            VK_V.0 as u8,
            0,
            KEYBD_EVENT_FLAGS(2), // KEYEVENTF_KEYUP
            0,
        );
        keybd_event(
            VK_CONTROL.0 as u8,
            0,
            KEYBD_EVENT_FLAGS(2), // KEYEVENTF_KEYUP
            0,
        );
    }
}

// C:\Users\user\Documents\file.txt
