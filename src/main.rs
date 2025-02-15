mod path;

use clipboard_win::{formats, get_clipboard, is_format_avail, set_clipboard, SysResult};
use eframe::egui::{self, Window};
use lazy_static::lazy_static;
use std::{
    sync::{
        mpsc::{channel, Receiver, Sender},
        Mutex,
    },
    thread,
};
use windows::core::PCWSTR;
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM},
    UI::{
        Input::KeyboardAndMouse::{
            keybd_event, GetAsyncKeyState, KEYBD_EVENT_FLAGS, VK_CONTROL, VK_LCONTROL, VK_RCONTROL,
            VK_SHIFT, VK_V,
        },
        WindowsAndMessaging::{
            CallNextHookEx, DispatchMessageW, FindWindowW, GetCursorPos, GetMessageW, SetWindowPos,
            SetWindowsHookExW, UnhookWindowsHookEx, HHOOK, HWND_TOPMOST, KBDLLHOOKSTRUCT, MSG,
            SWP_NOSIZE, SWP_NOZORDER, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP,
        },
    },
};

use crate::path::{ConvertablePath, PathType};

static mut HOOK_HANDLE: Option<HHOOK> = None;
lazy_static! {
    static ref BACKEND_TO_UI_SENDER: Mutex<Option<Sender<BackendToUiSignal>>> = Mutex::new(None);
}

enum BackendToUiCommand {
    ShowWindow,
    HideWindow,
    Select,
}

struct BackendToUiSignal {
    command: BackendToUiCommand,
    payload: Option<PathType>,
}

struct Pathte {
    recv_from_backend: Receiver<BackendToUiSignal>,
    send_to_backend: Sender<BackendToUiSignal>,
    window_open: bool,
    selected_path_type: PathType,
}

impl eframe::App for Pathte {
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
                BackendToUiCommand::Select => {
                    self.selected_path_type = signal.payload.unwrap();
                }
            }
        }

        Window::new("Pathte")
            .open(&mut self.window_open)
            .fade_out(true)
            .collapsible(false)
            .title_bar(false)
            .fixed_pos((10.0, 10.0))
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
    }
}

unsafe fn set_hook() {
    HOOK_HANDLE = Some(
        SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), None, 0)
            .expect("Failed to set hook"),
    );
}

unsafe fn unhook() {
    if let Some(hook) = HOOK_HANDLE {
        UnhookWindowsHookEx(hook);
    }
}

fn start_keyboard_hook_thread() {
    let _ = thread::spawn(move || unsafe {
        set_hook();

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND(0), 0, 0).as_bool() {
            DispatchMessageW(&msg);
        }

        unhook();
    });
}

fn main() {
    let (send_to_ui, recv_from_ui) = channel();
    let (send_to_backend, recv_from_backend) = channel();

    *BACKEND_TO_UI_SENDER.lock().unwrap() = Some(send_to_ui);

    start_keyboard_hook_thread();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_taskbar(false)
            .with_inner_size((300.0, 300.0))
            .with_position((100000.0, 100000.0))
            .with_transparent(true)
            .with_always_on_top(),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "Pathte",
        options.clone(),
        Box::new(move |_cc| {
            Ok(Box::new(Pathte {
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
        static mut SELECTED_PATH: Option<ConvertablePath> = None;

        match w_param.0 as u32 {
            WM_KEYDOWN => {
                if kb_struct.vkCode == VK_V.0 as u32 && ctrl_pressed {
                    if SELECTED_PATH.is_some() {
                        if let Some(sender) = BACKEND_TO_UI_SENDER.lock().unwrap().as_ref() {
                            let shift_pressed =
                                GetAsyncKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000 != 0;

                            SELECTED_PATH = if shift_pressed {
                                SELECTED_PATH.clone().map(|p| p.previous())
                            } else {
                                SELECTED_PATH.clone().map(|p| p.next())
                            };

                            let _ = sender.send(BackendToUiSignal {
                                command: BackendToUiCommand::Select,
                                payload: SELECTED_PATH.clone().map(|p| p.path_type()),
                            });
                        }

                        return LRESULT(1); // Prevent the default Ctrl+V behavior
                    } else if let Ok(text) = get_clipboard_text() {
                        SELECTED_PATH = match ConvertablePath::from_path(text.clone()) {
                            Ok(path) => Some(path),
                            Err(_) => None,
                        };

                        if SELECTED_PATH.is_some() {
                            if let Some(sender) = BACKEND_TO_UI_SENDER.lock().unwrap().as_ref() {
                                let _ = sender.send(BackendToUiSignal {
                                    command: BackendToUiCommand::Select,
                                    payload: SELECTED_PATH.clone().map(|p| p.path_type()),
                                });

                                let _ = sender.send(BackendToUiSignal {
                                    command: BackendToUiCommand::ShowWindow,
                                    payload: None,
                                });

                                // Set window position to cursor position
                                let hwnd = FindWindowW(
                                    None,
                                    PCWSTR::from_raw(
                                        "Pathte\0".encode_utf16().collect::<Vec<u16>>().as_ptr(),
                                    ),
                                );
                                let mut cursor_pos = POINT::default();
                                GetCursorPos(&mut cursor_pos);
                                SetWindowPos(
                                    hwnd,
                                    HWND_TOPMOST,
                                    cursor_pos.x,
                                    cursor_pos.y,
                                    0,
                                    0,
                                    SWP_NOSIZE | SWP_NOZORDER,
                                );

                                return LRESULT(1); // Prevent the default Ctrl+V behavior
                            }
                        }
                    }
                }
            }
            WM_KEYUP => {
                if (kb_struct.vkCode == VK_LCONTROL.0 as u32
                    || kb_struct.vkCode == VK_RCONTROL.0 as u32)
                    && SELECTED_PATH.is_some()
                {
                    if let Some(sender) = BACKEND_TO_UI_SENDER.lock().unwrap().as_ref() {
                        let _ = sender.send(BackendToUiSignal {
                            command: BackendToUiCommand::HideWindow,
                            payload: None,
                        });
                    }

                    let _ = paste_path(SELECTED_PATH.take().unwrap()); // TODO: Display errors

                    return LRESULT(1); // Prevent the default Ctrl+V behavior
                }
            }
            _ => {}
        }

        return CallNextHookEx(HOOK_HANDLE.unwrap_or(HHOOK(0)), code, w_param, l_param);
    }

    CallNextHookEx(HOOK_HANDLE.unwrap_or(HHOOK(0)), code, w_param, l_param)
}

fn paste_path(path: ConvertablePath) -> Result<(), String> {
    match get_clipboard_text() {
        Ok(original_path) => {
            set_clipboard_text(&path.to_string()).map_err(|e| e.to_string())?;
            unsafe {
                simulate_paste();
            }
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

fn get_clipboard_text() -> Result<String, String> {
    if !is_format_avail(formats::CF_UNICODETEXT) {
        return Err("Clipboard does not support unicode text.".to_string());
    }
    get_clipboard(formats::Unicode).map_err(|e| e.to_string())
}

fn set_clipboard_text(text: &str) -> SysResult<()> {
    set_clipboard(formats::Unicode, text)
}

unsafe fn simulate_paste() {
    unhook();
    keybd_event(VK_CONTROL.0 as u8, 0, KEYBD_EVENT_FLAGS(0), 0);
    keybd_event(VK_V.0 as u8, 0, KEYBD_EVENT_FLAGS(0), 0);
    keybd_event(VK_V.0 as u8, 0, KEYBD_EVENT_FLAGS(2), 0);
    keybd_event(VK_CONTROL.0 as u8, 0, KEYBD_EVENT_FLAGS(2), 0);
    set_hook();
}

// windows:   C:\Users\user\Documents\file.txt
// unix:      /home/user/Documents/file.txt
// wsl:       /mnt/c/Users/user/Documents/file.txt
//
//
