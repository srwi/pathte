use lazy_static::lazy_static;
use std::sync::Mutex;
use std::thread;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK,
    KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL,
};

pub type KeyboardEventHandler = fn(event_type: u32, kb_struct: &KBDLLHOOKSTRUCT) -> bool;

lazy_static! {
    static ref HOOK_HANDLE: Mutex<Option<HHOOK>> = Mutex::new(None);
    static ref EVENT_HANDLER: Mutex<Option<KeyboardEventHandler>> = Mutex::new(None);
}

pub fn set_keyboard_handler(handler: KeyboardEventHandler) {
    *EVENT_HANDLER.lock().unwrap() = Some(handler);
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
        let event_type = w_param.0 as u32;

        if let Some(handler) = *EVENT_HANDLER.lock().unwrap() {
            if handler(event_type, &kb_struct) {
                // Prevent original keypress from being processed
                return LRESULT(1);
            }
        }
    }

    // Leave original keypress to be processed by the system
    return CallNextHookEx(
        HOOK_HANDLE.lock().unwrap().unwrap_or(HHOOK(0)),
        code,
        w_param,
        l_param,
    );
}
