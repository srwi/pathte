use crate::path::{Path, PathType, UnixPath, WindowsPath, WslPath};

pub struct PathSelection {
    options: Vec<Box<dyn Path>>,
    current: usize,
}

#[derive(Clone)]
pub struct PathSelectionInfoEntry {
    pub label: String,
    pub path: String,
}

#[derive(Clone)]
pub struct PathSelectionInfo {
    pub options: Vec<PathSelectionInfoEntry>,
    pub selected: usize,
}

unsafe impl Send for PathSelection {}

impl PathSelection {
    pub fn new(raw_path: String) -> Option<Self> {
        let path = PathSelection::get_initial_path(raw_path)?;

        let options = vec![path.to_windows(), path.to_unix(), path.to_wsl()];
        let ok_options: Vec<Box<dyn Path>> = options
            .into_iter()
            .filter(|x| x.is_ok())
            .flatten()
            .collect();

        if ok_options.len() == 1 {
            // If there is only one option, there is nothing to select
            return None;
        }

        let initial_path_type = path.get_type();
        let initial_selection = ok_options
            .iter()
            .position(|x| x.get_type() == initial_path_type)
            .unwrap_or(0);

        Some(PathSelection {
            options: ok_options,
            current: initial_selection,
        })
    }

    pub fn next(&mut self) {
        self.current = (self.current + 1) % self.options.len();
    }

    pub fn previous(&mut self) {
        self.current = (self.current + self.options.len() - 1) % self.options.len();
    }

    pub fn get_selected_path_string(&self) -> String {
        self.options[self.current].as_string()
    }

    pub fn get_info(&self) -> PathSelectionInfo {
        let options = self
            .options
            .iter()
            .map(|x| PathSelectionInfoEntry {
                label: match x.get_type() {
                    PathType::Windows => "Win".to_string(),
                    PathType::Unix => "Unix".to_string(),
                    PathType::Wsl => "WSL".to_string(),
                },
                path: x.as_string(),
            })
            .collect();

        PathSelectionInfo {
            options,
            selected: self.current,
        }
    }

    fn get_initial_path(path: String) -> Option<Box<dyn Path>> {
        if let Ok(windows_path) = WindowsPath::new(path.clone()) {
            Some(Box::new(windows_path))
        } else if let Ok(unix_path) = UnixPath::new(path.clone()) {
            Some(Box::new(unix_path))
        } else if let Ok(wsl_path) = WslPath::new(path) {
            Some(Box::new(wsl_path))
        } else {
            None
        }
    }
}
