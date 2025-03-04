use crate::GUI_SENDER;
use crate::PATH_SELECTION;

use lazy_static::lazy_static;
use std::sync::Mutex;
use std::thread;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_CONTROL, VK_LCONTROL, VK_RCONTROL, VK_SHIFT, VK_V,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK,
    KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP,
};

use crate::clipboard;
use crate::path_selection::PathSelection;
use crate::win_api;

lazy_static! {
    static ref HOOK_HANDLE: Mutex<Option<HHOOK>> = Mutex::new(None);
}

pub fn set_hook() {
    let handle = unsafe {
        SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), None, 0)
            .expect("Failed to set keyboard hook.")
    };

    *HOOK_HANDLE.lock().unwrap() = Some(handle);
}

pub fn unhook() {
    if let Some(hook) = *HOOK_HANDLE.lock().unwrap() {
        unsafe {
            UnhookWindowsHookEx(hook);
        }
    }
}

pub fn start_keyboard_hook_thread() {
    let _ = thread::spawn(move || {
        set_hook();

        let mut msg = MSG::default();
        unsafe {
            while GetMessageW(&mut msg, HWND(0), 0, 0).as_bool() {
                DispatchMessageW(&msg);
            }
        }

        unhook();
    });
}

unsafe extern "system" fn keyboard_hook_proc(
    code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if code >= 0 {
        let kb_struct = *(l_param.0 as *const KBDLLHOOKSTRUCT);
        let ctrl_pressed = GetAsyncKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000 != 0;
        let mut path_selection = PATH_SELECTION.lock().unwrap();

        match w_param.0 as u32 {
            WM_KEYDOWN => {
                if kb_struct.vkCode == VK_V.0 as u32 && ctrl_pressed {
                    if path_selection.is_some() {
                        if let Some(sender) = GUI_SENDER.lock().unwrap().as_ref() {
                            let shift_pressed =
                                GetAsyncKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000 != 0;

                            if shift_pressed {
                                path_selection.as_mut().unwrap().previous();
                            } else {
                                path_selection.as_mut().unwrap().next();
                            };

                            let _ = sender.send(path_selection.as_ref().map(|ps| ps.get_info()));
                        }

                        return LRESULT(1); // Prevent the default Ctrl+V behavior
                    } else if let Ok(text) = clipboard::get_clipboard_text() {
                        *path_selection = PathSelection::new(text);

                        if path_selection.is_some() {
                            if let Some(sender) = GUI_SENDER.lock().unwrap().as_ref() {
                                let _ =
                                    sender.send(path_selection.as_ref().map(|ps| ps.get_info()));

                                if let Ok(hwnd) = win_api::find_app_window() {
                                    win_api::move_window_to_cursor(hwnd).unwrap();
                                }

                                return LRESULT(1); // Prevent the default Ctrl+V behavior
                            }
                        }
                    }
                }
            }
            WM_KEYUP => {
                if (kb_struct.vkCode == VK_LCONTROL.0 as u32
                    || kb_struct.vkCode == VK_RCONTROL.0 as u32)
                    && path_selection.is_some()
                {
                    if let Some(sender) = GUI_SENDER.lock().unwrap().as_ref() {
                        let _ = sender.send(None);
                    }

                    let path = path_selection.take().unwrap().get_selected_path_string();
                    let _ = clipboard::paste_path(path); // TODO: Display errors

                    return LRESULT(1); // Prevent the default Ctrl+V behavior
                }
            }
            _ => {}
        }

        return CallNextHookEx(
            HOOK_HANDLE.lock().unwrap().unwrap_or(HHOOK(0)),
            code,
            w_param,
            l_param,
        );
    }

    CallNextHookEx(
        HOOK_HANDLE.lock().unwrap().unwrap_or(HHOOK(0)),
        code,
        w_param,
        l_param,
    )
}
