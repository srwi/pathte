use eframe::egui;
use global_hotkey::{hotkey, GlobalHotKeyEvent, GlobalHotKeyManager};
use std::sync::{Arc, Mutex};
use std::thread;
use tray_icon::{
    menu::{Menu, MenuItem},
    Icon, TrayIconBuilder,
};

struct MyApp {
    show_window: Arc<Mutex<bool>>,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Hello, world!");
            if ui.button("Close").clicked() {
                *self.show_window.lock().unwrap() = false;
            }
        });
    }
}

fn main() {
    let show_window = Arc::new(Mutex::new(false));
    let show_window_clone = Arc::clone(&show_window);

    let hotkey_manager = GlobalHotKeyManager::new().unwrap();
    // hotkey_manager.register(hotkey!(Ctrl, Shift, F1)).unwrap();

    let _ = thread::spawn(move || loop {
        // if let Ok(event) = hotkey_manager.receiver().try_recv() {
        // if let GlobalHotKeyEvent::Pressed { .. } = event {
        //     *show_window_clone.lock().unwrap() = true;
        // }
        // }
        thread::sleep(std::time::Duration::from_millis(100));
    });

    let tray_menu = Menu::new();
    let _ = tray_menu.insert(&MenuItem::new("Quit", true, None), 0);
    let _tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip("Pathte")
        .build()
        .unwrap();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([300f32, 200f32]),
        ..Default::default()
    };

    loop {
        if *show_window.lock().unwrap() {
            let _ = eframe::run_native(
                "Pathte",
                options.clone(),
                Box::new(|_cc| {
                    Ok(Box::new(MyApp {
                        show_window: Arc::clone(&show_window),
                    }))
                }),
            );
            *show_window.lock().unwrap() = false; // Reset the flag after closing the window
        }
        thread::sleep(std::time::Duration::from_millis(100));
    }
}
