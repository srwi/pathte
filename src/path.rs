use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref WSL_REGEX: Regex =
        Regex::new(r#"^/mnt/(([A-Za-z]/?$)|([A-Za-z]/[^\x00]*))$"#).unwrap();
    static ref WINDOWS_REGEX: Regex =
        Regex::new(r#"^([a-zA-Z]:\\?$)|([^\x00-\x1F<>:"|?*/]*\\[^\x00-\x1F<>:"|?*/]*$)"#).unwrap();
    static ref UNIX_REGEX: Regex = Regex::new(r"^[^\x00]*/[^\x00]*$").unwrap();
    static ref PROTOCOL_REGEX: Regex = Regex::new(r"(http|https|ftp|sftp|file):$").unwrap();
}

pub trait Path {
    fn to_windows(&self) -> Result<Box<dyn Path>, String>;
    fn to_unix(&self) -> Result<Box<dyn Path>, String>;
    fn to_wsl(&self) -> Result<Box<dyn Path>, String>;
    fn as_string(&self) -> String;
    fn get_type(&self) -> PathType;
}

#[derive(Clone)]
pub struct WindowsPath {
    path: String,
}

#[derive(Clone)]
pub struct UnixPath {
    path: String,
}

#[derive(Clone)]
pub struct WslPath {
    path: String,
}

impl WindowsPath {
    pub fn new(path: String) -> Result<Self, String> {
        if WINDOWS_REGEX.is_match(&path) {
            Ok(WindowsPath { path })
        } else {
            Err("The given path is not a Windows path.".to_string())
        }
    }
}

impl UnixPath {
    pub fn new(path: String) -> Result<Self, String> {
        if UNIX_REGEX.is_match(&path) {
            Ok(UnixPath { path })
        } else {
            Err("The given path is not a Unix path.".to_string())
        }
    }
}

impl WslPath {
    pub fn new(path: String) -> Result<Self, String> {
        if WSL_REGEX.is_match(&path) {
            Ok(WslPath { path })
        } else {
            Err("The given path is not a WSL path.".to_string())
        }
    }
}

impl Path for WindowsPath {
    fn to_windows(&self) -> Result<Box<dyn Path>, String> {
        Ok(Box::new(self.clone()))
    }

    fn to_unix(&self) -> Result<Box<dyn Path>, String> {
        let unix_path = self.path.replace('\\', "/");
        match UnixPath::new(unix_path) {
            Ok(path) => Ok(Box::new(path)),
            Err(e) => Err(e),
        }
    }

    fn to_wsl(&self) -> Result<Box<dyn Path>, String> {
        let drive_regex = Regex::new(r"^([A-Za-z]):").unwrap();
        let wsl_path = drive_regex
            .replace(&self.path, |captures: &regex::Captures| {
                format!("/mnt/{}", &captures[1].to_lowercase())
            })
            .replace("\\", "/");
        match WslPath::new(wsl_path) {
            Ok(path) => Ok(Box::new(path)),
            Err(e) => Err(e),
        }
    }

    fn as_string(&self) -> String {
        self.path.clone()
    }

    fn get_type(&self) -> PathType {
        PathType::Windows
    }
}

impl Path for UnixPath {
    fn to_windows(&self) -> Result<Box<dyn Path>, String> {
        let windows_path = self.path.replace('/', "\\");
        match WindowsPath::new(windows_path) {
            Ok(path) => Ok(Box::new(path)),
            Err(e) => Err(e),
        }
    }

    fn to_unix(&self) -> Result<Box<dyn Path>, String> {
        Ok(Box::new(self.clone()))
    }

    fn to_wsl(&self) -> Result<Box<dyn Path>, String> {
        let wsl_path = self.path.clone();
        match WslPath::new(wsl_path) {
            Ok(path) => Ok(Box::new(path)),
            Err(e) => Err(e),
        }
    }

    fn as_string(&self) -> String {
        self.path.clone()
    }

    fn get_type(&self) -> PathType {
        PathType::Unix
    }
}

impl Path for WslPath {
    fn to_windows(&self) -> Result<Box<dyn Path>, String> {
        let drive_regex = Regex::new(r"^/mnt/([A-Za-z])").unwrap();
        let windows_path = drive_regex
            .replace(&self.path, |captures: &regex::Captures| {
                format!("{}:", &captures[1].to_uppercase())
            })
            .replace('/', "\\");
        match WindowsPath::new(windows_path) {
            Ok(path) => Ok(Box::new(path)),
            Err(e) => Err(e),
        }
    }

    fn to_unix(&self) -> Result<Box<dyn Path>, String> {
        let unix_path = self.path.clone();
        match UnixPath::new(unix_path) {
            Ok(path) => Ok(Box::new(path)),
            Err(e) => Err(e),
        }
    }

    fn to_wsl(&self) -> Result<Box<dyn Path>, String> {
        Ok(Box::new(self.clone()))
    }

    fn as_string(&self) -> String {
        self.path.clone()
    }

    fn get_type(&self) -> PathType {
        PathType::Wsl
    }
}

#[derive(Debug, PartialEq)]
pub enum PathType {
    Windows,
    Unix,
    Wsl,
}
