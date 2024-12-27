use clipboard_win::{formats, get_clipboard, is_format_avail, set_clipboard, SysResult};
use eframe::egui::{self, Window};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use tray_icon::{
    menu::{Menu, MenuItem},
    TrayIconBuilder,
};
use std::sync::mpsc::{Receiver, channel, Sender};
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

enum BackendToUiCommand {
    ShowWindow,
    HideWindow,
    SelectNext,
    SelectPrevious,
}

struct BackendToUiSignal {
    command: BackendToUiCommand,
    payload: Option<i32>,
}

enum UiToBackendSignal {
    Select,
    Cancel,
}

struct UiToBackendCommand {
    command: BackendToUiCommand,
    payload: Option<i32>,
}

struct MyApp {
    recv_from_backend: Receiver<BackendToUiSignal>,
    send_to_backend: Sender<BackendToUiSignal>,
    window_open: bool,
}

impl eframe::App for MyApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ctx.send_viewport_cmd(egui::ViewportCommand::MousePassthrough(true));

        if self.recv_from_backend.try_recv().is_ok() {
            self.window_open = true;
        }

        Window::new("Pathte")
            .open(&mut self.window_open)
            .fade_out(true)
            .show(ctx, |ui| {
                if (self.recv_from_backend.try_recv().is_ok()) {
                    ui.label("Received from backend");
                }
            });

        ctx.request_repaint();
    }
}

fn main() {
    let (send_to_ui, recv_from_ui) = channel();
    let (send_to_backend, recv_from_backend) = channel();

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
    });
    
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            // .with_decorations(false)
            // .with_taskbar(false)
            .with_maximized(true),
            // .with_transparent(true),
            // .with_always_on_top(),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "Pathte",
        options.clone(),
        Box::new(move |_cc| {
            Ok(Box::new(MyApp {
                recv_from_backend: recv_from_ui,
                send_to_backend: send_to_backend,
                window_open: false,
            }))
        }),
    );
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
