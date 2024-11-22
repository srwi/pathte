use eframe::{App, CreationContext, Frame, NativeOptions};
use egui::{CentralPanel, Context, Vec2};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

pub struct ListSelectionDialog<T: 'static + Clone + Send> {
    title: String,
    items: Vec<T>,
    formatter: Box<dyn Fn(&T) -> String + Send>,
    result_sender: Option<Sender<Option<T>>>,
}

struct DialogWindow<T: 'static + Clone + Send> {
    title: String,
    items: Vec<T>,
    formatter: Box<dyn Fn(&T) -> String + Send>,
    result_sender: Sender<Option<T>>,
}

impl<T: 'static + Clone + Send> App for DialogWindow<T> {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        CentralPanel::default().show(ctx, |_| {
            egui::Window::new(&self.title)
                .collapsible(false)
                .resizable(true)
                .default_size(Vec2::new(300.0, 400.0))
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for item in &self.items {
                            let text = (self.formatter)(item);
                            if ui.button(&text).clicked() {
                                self.result_sender.send(Some(item.clone())).ok();
                                _frame.close();
                            }
                        }
                    });

                    ui.separator();
                    if ui.button("Cancel").clicked() {
                        self.result_sender.send(None).ok();
                        _frame.close();
                    }
                });
        });
    }
}

impl<T: 'static + Clone + Send> ListSelectionDialog<T> {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            items: Vec::new(),
            formatter: Box::new(|item| format!("{:?}", item)),
            result_sender: None,
        }
    }

    pub fn with_items(mut self, items: Vec<T>) -> Self {
        self.items = items;
        self
    }

    pub fn with_formatter(mut self, formatter: impl Fn(&T) -> String + Send + 'static) -> Self {
        self.formatter = Box::new(formatter);
        self
    }

    pub fn show(self) -> Receiver<Option<T>> {
        let (sender, receiver) = channel();

        let window = DialogWindow {
            title: self.title,
            items: self.items,
            formatter: self.formatter,
            result_sender: sender,
        };

        let options = NativeOptions {
            initial_window_size: Some(Vec2::new(300.0, 400.0)),
            resizable: true,
            ..Default::default()
        };

        thread::spawn(move || {
            eframe::run_native(options, Box::new(|_cc| Box::new(window))).ok();
        });

        receiver
    }
}
