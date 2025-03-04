#![windows_subsystem = "windows"]

mod clipboard;
mod keyboard_hook;
mod path;
mod path_selection;
mod tray;
mod win_api;

use eframe::egui::{self, Window};
use lazy_static::lazy_static;
use path_selection::{PathSelection, PathSelectionInfo};
use std::sync::{
    mpsc::{channel, Receiver, Sender},
    Mutex,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_CONTROL, VK_LCONTROL, VK_RCONTROL, VK_SHIFT, VK_V,
};
use windows::Win32::UI::WindowsAndMessaging::{KBDLLHOOKSTRUCT, WM_KEYDOWN, WM_KEYUP};

lazy_static! {
    static ref GUI_SENDER: Mutex<Option<Sender<Option<PathSelectionInfo>>>> = Mutex::new(None);
    static ref PATH_SELECTION: Mutex<Option<PathSelection>> = Mutex::new(None);
}

static APP_NAME: &str = "Pathte";

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

        Window::new(APP_NAME)
            .open(&mut self.current_path_selection_info.is_some())
            .fade_out(true)
            .collapsible(false)
            .title_bar(false)
            .fixed_pos((10.0, 10.0))
            .resizable(false)
            .show(ctx, |ui| {
                egui::Grid::new("path_grid")
                    .spacing([-5.0, 0.0])
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

fn main() {
    let (gui_sender, gui_receiver) = channel();
    *GUI_SENDER.lock().unwrap() = Some(gui_sender);

    let _tray_icon = tray::create_tray_icon();

    keyboard_hook::set_keyboard_handler(handle_keyboard_event);
    keyboard_hook::start_keyboard_hook_thread();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_taskbar(false)
            .with_inner_size((1000.0, 300.0))
            .with_position((100000.0, 100000.0))
            .with_transparent(true)
            .with_always_on_top(),
        ..Default::default()
    };
    let _ = eframe::run_native(
        APP_NAME,
        options.clone(),
        Box::new(move |_cc| {
            Ok(Box::new(Pathte {
                signal_receiver: gui_receiver,
                current_path_selection_info: None,
            }))
        }),
    );
}

fn handle_keyboard_event(event_type: u32, kb_struct: &KBDLLHOOKSTRUCT) -> bool {
    let ctrl_pressed = unsafe { GetAsyncKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000 != 0 };
    let mut path_selection = PATH_SELECTION.lock().unwrap();

    match event_type {
        WM_KEYDOWN => {
            if kb_struct.vkCode == VK_V.0 as u32 && ctrl_pressed {
                if let Some(ref mut selection) = *path_selection {
                    // Handle Ctrl + V when a path is already selected
                    if let Some(sender) = GUI_SENDER.lock().unwrap().as_ref() {
                        let shift_pressed =
                            unsafe { GetAsyncKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000 != 0 };

                        if shift_pressed {
                            selection.previous();
                        } else {
                            selection.next();
                        }

                        let _ = sender.send(Some(selection.get_info()));
                    }
                    return true;
                } else if let Ok(text) = clipboard::get_clipboard_text() {
                    // Handle Ctrl + V when no path is selected
                    *path_selection = PathSelection::new(text);

                    if let Some(ref selection) = *path_selection {
                        if let Some(sender) = GUI_SENDER.lock().unwrap().as_ref() {
                            let _ = sender.send(Some(selection.get_info()));

                            if let Ok(hwnd) = win_api::find_app_window() {
                                let _ = win_api::move_window_to_cursor(hwnd);
                            }

                            return true;
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
                // Handle Ctrl release (paste the selected path)
                if let Some(sender) = GUI_SENDER.lock().unwrap().as_ref() {
                    let _ = sender.send(None);
                }

                let path = path_selection.take().unwrap().get_selected_path_string();
                let _ = clipboard::paste_path(path);

                return true;
            }
        }
        _ => {}
    }

    false // Don't intercept by default
}
