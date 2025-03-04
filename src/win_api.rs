use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, GetCursorPos, SetWindowPos, HWND_TOPMOST, SWP_NOSIZE, SWP_NOZORDER,
};

pub fn find_app_window() -> Result<HWND, String> {
    unsafe {
        let hwnd = FindWindowW(
            None,
            PCWSTR::from_raw("Pathte\0".encode_utf16().collect::<Vec<u16>>().as_ptr()),
        );

        if hwnd.0 == 0 {
            Err("Failed to find window".to_string())
        } else {
            Ok(hwnd)
        }
    }
}

pub fn move_window_to_cursor(hwnd: HWND) -> Result<(), String> {
    unsafe {
        let mut cursor_pos = POINT::default();
        if !GetCursorPos(&mut cursor_pos).as_bool() {
            return Err("Failed to get cursor position".to_string());
        }

        SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            cursor_pos.x,
            cursor_pos.y,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER,
        );

        Ok(())
    }
}
