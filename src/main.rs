use clipboard_win::{formats, get_clipboard, is_format_avail, set_clipboard, SysResult};
use eframe::egui::{self, Window};
use lazy_static::lazy_static;
use std::path::Path;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Mutex;
use std::thread;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    keybd_event, GetAsyncKeyState, KEYBD_EVENT_FLAGS, VK_CONTROL, VK_LCONTROL, VK_RCONTROL,
    VK_SHIFT, VK_V,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK,
    KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP,
};

static mut HOOK_HANDLE: Option<HHOOK> = None;
lazy_static! {
    static ref BACKEND_TO_UI_SENDER: Mutex<Option<Sender<BackendToUiSignal>>> = Mutex::new(None);
}

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

#[derive(PartialEq)]
enum PathType {
    Windows,
    Unix,
    WSL,
}

impl PathType {
    fn next(&self) -> PathType {
        match self {
            PathType::Windows => PathType::Unix,
            PathType::Unix => PathType::WSL,
            PathType::WSL => PathType::Windows,
        }
    }

    fn previous(&self) -> PathType {
        match self {
            PathType::Windows => PathType::WSL,
            PathType::Unix => PathType::Windows,
            PathType::WSL => PathType::Unix,
        }
    }
}

enum BackendToUiCommand {
    ShowWindow,
    HideWindow,
    Select,
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
    selected_path_type: PathType,
}

impl eframe::App for MyApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.send_viewport_cmd(egui::ViewportCommand::MousePassthrough(true));

        if let Ok(signal) = self.recv_from_backend.try_recv() {
            match signal.command {
                BackendToUiCommand::ShowWindow => {
                    self.window_open = true;
                }
                BackendToUiCommand::HideWindow => {
                    self.window_open = false;
                }
                BackendToUiCommand::SelectNext => {
                    self.selected_path_type = self.selected_path_type.next();
                }
                BackendToUiCommand::SelectPrevious => {
                    self.selected_path_type = self.selected_path_type.previous();
                }
                BackendToUiCommand::Select => {
                    self.selected_path_type = match signal.payload {
                        Some(0) => PathType::Windows,
                        Some(1) => PathType::Unix,
                        Some(2) => PathType::WSL,
                        _ => panic!("Invalid path type"),
                    };
                }
            }
        }

        Window::new("Pathte")
            .open(&mut self.window_open)
            .fade_out(true)
            .collapsible(false)
            .title_bar(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.selectable_value(
                        &mut self.selected_path_type,
                        PathType::Windows,
                        "Windows path",
                    );
                    ui.selectable_value(&mut self.selected_path_type, PathType::Unix, "Unix path");
                    ui.selectable_value(&mut self.selected_path_type, PathType::WSL, "WSL path");
                });
            });

        ctx.request_repaint();
    }
}

fn main() {
    let (send_to_ui, recv_from_ui) = channel();
    let (send_to_backend, recv_from_backend) = channel();

    *BACKEND_TO_UI_SENDER.lock().unwrap() = Some(send_to_ui);

    let _ = thread::spawn(move || unsafe {
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
    });

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_taskbar(false)
            .with_maximized(true)
            .with_transparent(true)
            .with_always_on_top(),
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
                selected_path_type: PathType::Unix,
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
        static mut SELECTION_WINDOW_VISIBLE: bool = false;

        match w_param.0 as u32 {
            WM_KEYDOWN => {
                if kb_struct.vkCode == VK_V.0 as u32 && ctrl_pressed {
                    let clipboard_text = get_clipboard_text();

                    if SELECTION_WINDOW_VISIBLE {
                        let shift_pressed =
                            GetAsyncKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000 != 0;
                        if shift_pressed {
                            if let Some(sender) = BACKEND_TO_UI_SENDER.lock().unwrap().as_ref() {
                                let _ = sender.send(BackendToUiSignal {
                                    command: BackendToUiCommand::SelectPrevious,
                                    payload: None,
                                });
                            }
                        } else {
                            if let Some(sender) = BACKEND_TO_UI_SENDER.lock().unwrap().as_ref() {
                                let _ = sender.send(BackendToUiSignal {
                                    command: BackendToUiCommand::SelectNext,
                                    payload: None,
                                });
                            }
                        }
                    } else if let Ok(text) = clipboard_text {
                        let path_type = get_path_type(&text);
                        if path_type.is_some() {
                            if let Some(sender) = BACKEND_TO_UI_SENDER.lock().unwrap().as_ref() {
                                let _ = sender.send(BackendToUiSignal {
                                    command: BackendToUiCommand::Select,
                                    payload: Some(path_type.unwrap() as i32),
                                });
                                let _ = sender.send(BackendToUiSignal {
                                    command: BackendToUiCommand::ShowWindow,
                                    payload: None,
                                });
                                SELECTION_WINDOW_VISIBLE = true;
                            }
                        }
                    }

                    return LRESULT(1); // Prevent the default Ctrl+V behavior
                }
            }
            WM_KEYUP => {
                if (kb_struct.vkCode == VK_LCONTROL.0 as u32
                    || kb_struct.vkCode == VK_RCONTROL.0 as u32)
                    && SELECTION_WINDOW_VISIBLE
                {
                    if let Some(sender) = BACKEND_TO_UI_SENDER.lock().unwrap().as_ref() {
                        let _ = sender.send(BackendToUiSignal {
                            command: BackendToUiCommand::HideWindow,
                            payload: None,
                        });
                    }

                    if let Err(e) = handle_hotkey() {
                        eprintln!("Error: {}", e);
                    }

                    SELECTION_WINDOW_VISIBLE = false;
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

fn get_path_type(text: &str) -> Option<PathType> {
    if text.contains('\\') {
        Some(PathType::Windows)
    } else if text.starts_with("/mnt/c/") {
        Some(PathType::WSL)
    } else if text.contains('/') {
        Some(PathType::Unix)
    } else {
        None
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

// windows:   C:\Users\user\Documents\file.txt
// unix:      /home/user/Documents/file.txt
// wsl:       /mnt/c/Users/user/Documents/file.txt
//
//
