mod path;
mod path_selection;
mod tray;

use clipboard_win::{formats, get_clipboard, is_format_avail, set_clipboard, SysResult};
use eframe::egui::{self, Window};
use lazy_static::lazy_static;
use path_selection::{PathSelection, PathSelectionInfo};
use std::sync::{
    mpsc::{channel, Receiver, Sender},
    Mutex,
};
use std::thread;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    keybd_event, GetAsyncKeyState, KEYBD_EVENT_FLAGS, VK_CONTROL, VK_LCONTROL, VK_RCONTROL,
    VK_SHIFT, VK_V,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, FindWindowW, GetCursorPos, GetMessageW, SetWindowPos,
    SetWindowsHookExW, UnhookWindowsHookEx, HHOOK, HWND_TOPMOST, KBDLLHOOKSTRUCT, MSG, SWP_NOSIZE,
    SWP_NOZORDER, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP,
};

lazy_static! {
    static ref HOOK_HANDLE: Mutex<Option<HHOOK>> = Mutex::new(None);
    static ref GUI_SENDER: Mutex<Option<Sender<Option<PathSelectionInfo>>>> = Mutex::new(None);
    static ref PATH_SELECTION: Mutex<Option<PathSelection>> = Mutex::new(None);
}

struct Pathte {
    signal_receiver: Receiver<Option<PathSelectionInfo>>,
    current_path_selection_info: Option<PathSelectionInfo>,
}

impl eframe::App for Pathte {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.send_viewport_cmd(egui::ViewportCommand::MousePassthrough(true));

        if let Ok(path_selection_info) = self.signal_receiver.try_recv() {
            self.current_path_selection_info = path_selection_info;
        }

        Window::new("Pathte")
            .open(&mut self.current_path_selection_info.is_some())
            .fade_out(true)
            .collapsible(false)
            .title_bar(false)
            .fixed_pos((10.0, 10.0))
            .resizable(false)
            .show(ctx, |ui| {
                egui::Grid::new("path_grid")
                    .spacing([-5.0, 0.0]) // Adjust the spacing between columns
                    .show(ui, |ui| {
                        if let Some(info) = &mut self.current_path_selection_info {
                            for (index, option) in info.options.iter().enumerate() {
                                ui.label(&option.label);
                                ui.selectable_value(&mut info.selected, index, &option.path);
                                ui.end_row();
                            }
                        }
                    });
            });
    }
}

fn set_hook() {
    let handle = unsafe {
        SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), None, 0)
            .expect("Failed to set keyboard hook.")
    };

    *HOOK_HANDLE.lock().unwrap() = Some(handle);
}

fn unhook() {
    if let Some(hook) = *HOOK_HANDLE.lock().unwrap() {
        unsafe {
            UnhookWindowsHookEx(hook);
        }
    }
}

fn start_keyboard_hook_thread() {
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

fn main() {
    let (gui_sender, gui_receiver) = channel();
    *GUI_SENDER.lock().unwrap() = Some(gui_sender);

    let _tray_icon = tray::create_tray_icon();

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
                signal_receiver: gui_receiver,
                current_path_selection_info: None,
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
                    } else if let Ok(text) = get_clipboard_text() {
                        *path_selection = PathSelection::new(text);

                        if path_selection.is_some() {
                            if let Some(sender) = GUI_SENDER.lock().unwrap().as_ref() {
                                let _ =
                                    sender.send(path_selection.as_ref().map(|ps| ps.get_info()));

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
                    && path_selection.is_some()
                {
                    if let Some(sender) = GUI_SENDER.lock().unwrap().as_ref() {
                        let _ = sender.send(None);
                    }

                    let path = path_selection.take().unwrap().get_selected_path_string();
                    let _ = paste_path(path); // TODO: Display errors

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

fn paste_path(path: String) -> Result<(), String> {
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

fn get_clipboard_text() -> Result<String, String> {
    if !is_format_avail(formats::CF_UNICODETEXT) {
        return Err("Clipboard does not support unicode text.".to_string());
    }
    get_clipboard(formats::Unicode).map_err(|e| e.to_string())
}

fn set_clipboard_text(text: &str) -> SysResult<()> {
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

// windows:   C:\Users\user\Documents\file.txt
// unix:      /home/user/Documents/file.txt
// wsl:       /mnt/c/Users/user/Documents/file.txt
//
//C:/Users/user/Documents/file.txt
