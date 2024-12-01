use clipboard_win::{formats, get_clipboard, is_format_avail, set_clipboard, SysResult};
use eframe::egui;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use tray_icon::{
    menu::{Menu, MenuItem},
    TrayIconBuilder,
};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    keybd_event, GetAsyncKeyState, KEYBD_EVENT_FLAGS, VK_CONTROL, VK_V,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK,
    KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP,
};

static mut HOOK_HANDLE: Option<HHOOK> = None;
static mut PREV_CTRL_STATE: bool = false;

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

struct MyApp {
    show_window: Arc<Mutex<bool>>,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let window_info = _frame.info().clone();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Hello, world!");
            if ui.button("Close").clicked() {
                *self.show_window.lock().unwrap() = false;
            }
        });
    }
}

fn main() {
    let show_window = Arc::new(Mutex::new(true));
    let show_window_condvar = Arc::new(std::sync::Condvar::new());

    // Clone for the hook thread
    let show_window_clone_for_thread = Arc::clone(&show_window);
    let show_window_condvar_clone = Arc::clone(&show_window_condvar);
    let _ = thread::spawn(move || {
        unsafe {
            HOOK_HANDLE = Some(
                SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), None, 0)
                    .expect("Failed to set hook"),
            );

            let mut msg = MSG::default();
            while GetMessageW(&mut msg, HWND(0), 0, 0).as_bool() {
                DispatchMessageW(&msg);
            }

            if let Some(hook) = HOOK_HANDLE {
                UnhookWindowsHookEx(hook);
            }
        }

        // Simulate triggering the UI thread to show the window
        {
            let mut show_window = show_window_clone_for_thread.lock().unwrap();
            *show_window = true;
            show_window_condvar_clone.notify_one();
        }
    });

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([300f32, 200f32]),
        ..Default::default()
    };

    // Main loop
    let show_window_for_main = Arc::clone(&show_window);
    loop {
        // Wait until the window should be shown
        let mut show_window = show_window_condvar
            .wait_while(show_window_for_main.lock().unwrap(), |&mut show| !show)
            .unwrap();

        // If show_window is true, launch the GUI
        let show_window_clone_for_app = Arc::clone(&show_window_for_main);
        let result = eframe::run_native(
            "Pathte",
            options.clone(),
            Box::new(move |_cc| {
                Ok(Box::new(MyApp {
                    show_window: show_window_clone_for_app,
                }))
            }),
        );

        if result.is_err() {
            eprintln!("Error running eframe: {:?}", result);
            break;
        }

        *show_window = false;

        // Close the window using win32 DestroyWindow
        unsafe {
            let hwnd = GetConsoleWindow();
            if !hwnd.is_null() {
                ShowWindow(hwnd, winapi::um::winuser::SW_HIDE);
                let _ = windows::Win32::UI::WindowsAndMessaging::DestroyWindow(hwnd);
            }
        }
        // There is a possible workaround described here: https://github.com/emilk/egui/pull/1889
    }
}

unsafe extern "system" fn keyboard_hook_proc(
    code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if code >= 0 {
        let kb_struct = *(l_param.0 as *const KBDLLHOOKSTRUCT);
        let ctrl_pressed = GetAsyncKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000 != 0;

        match w_param.0 as u32 {
            WM_KEYDOWN => {
                if kb_struct.vkCode == VK_V.0 as u32 && ctrl_pressed && !PREV_CTRL_STATE {
                    PREV_CTRL_STATE = true;
                    if let Err(e) = handle_hotkey() {
                        eprintln!("Error: {}", e);
                    }
                    return LRESULT(1); // Prevent the default Ctrl+V behavior
                }
            }
            WM_KEYUP => {
                if kb_struct.vkCode == VK_CONTROL.0 as u32 {
                    PREV_CTRL_STATE = false;
                }
            }
            _ => {}
        }
    }

    CallNextHookEx(HOOK_HANDLE.unwrap_or(HHOOK(0)), code, w_param, l_param)
}

fn handle_hotkey() -> Result<(), ClipboardError> {
    match get_clipboard_text() {
        Ok(text) => {
            if let Some(converted) = convert_if_valid_path(&text) {
                set_clipboard_text(&converted)
                    .map_err(|e| ClipboardError::ClipboardError(e.to_string()))?;
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
