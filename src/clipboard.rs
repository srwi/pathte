use clipboard_win::{formats, get_clipboard, is_format_avail, set_clipboard, SysResult};
use std::thread;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    keybd_event, KEYBD_EVENT_FLAGS, VK_CONTROL, VK_V,
};

use crate::keyboard_hook::{set_hook, unhook};

pub fn paste_path(path: String) -> Result<(), String> {
    match get_clipboard_text() {
        Ok(original_path) => {
            set_clipboard_text(&path).map_err(|e| e.to_string())?;
            simulate_paste();
            thread::spawn(move || {
                // The simulated keypresses take some time to register, so we wait a bit before restoring the clipboard
                thread::sleep(std::time::Duration::from_millis(100));
                let _ = set_clipboard_text(&original_path);
            });
        }
        Err(e) => return Err(e),
    }
    Ok(())
}

pub fn get_clipboard_text() -> Result<String, String> {
    if !is_format_avail(formats::CF_UNICODETEXT) {
        return Err("Clipboard does not support unicode text.".to_string());
    }
    get_clipboard(formats::Unicode).map_err(|e| e.to_string())
}

pub fn set_clipboard_text(text: &str) -> SysResult<()> {
    set_clipboard(formats::Unicode, text)
}

fn simulate_paste() {
    unhook();
    unsafe {
        keybd_event(VK_CONTROL.0 as u8, 0, KEYBD_EVENT_FLAGS(0), 0);
        keybd_event(VK_V.0 as u8, 0, KEYBD_EVENT_FLAGS(0), 0);
        keybd_event(VK_V.0 as u8, 0, KEYBD_EVENT_FLAGS(2), 0);
        keybd_event(VK_CONTROL.0 as u8, 0, KEYBD_EVENT_FLAGS(2), 0);
    }
    set_hook();
}
