use lazy_static::lazy_static;
use regex::Regex;

#[derive(Debug, PartialEq)]
pub enum PathType {
    Windows,
    Unix,
    WSL,
}

#[derive(Debug, Clone)]
pub enum ConvertablePath {
    Windows(WindowsPath),
    Unix(UnixPath),
    WSL(WslPath),
}

#[derive(Debug, Clone)]
pub struct WindowsPath {
    path: String,
}

#[derive(Debug, Clone)]
pub struct UnixPath {
    path: String,
}

#[derive(Debug, Clone)]
pub struct WslPath {
    path: String,
}

lazy_static! {
    static ref WSL_REGEX: Regex =
        Regex::new(r#"^/mnt/(([A-Za-z]/?$)|([A-Za-z]/[^\x00]*))$"#).unwrap();
    static ref WINDOWS_REGEX: Regex =
        Regex::new(r#"^([a-zA-Z]:\\?$)|([^\x00-\x1F<>:"|?*/]*\\[^\x00-\x1F<>:"|?*/]*$)"#).unwrap();
    static ref UNIX_REGEX: Regex = Regex::new(r"^[^\x00]*/[^\x00]*$").unwrap();
}

impl ConvertablePath {
    pub fn from_path(path: String) -> Result<Self, String> {
        if WSL_REGEX.is_match(&path) {
            Ok(ConvertablePath::WSL(WslPath::new(path)))
        } else if UNIX_REGEX.is_match(&path) {
            Ok(ConvertablePath::Unix(UnixPath::new(path)))
        } else if WINDOWS_REGEX.is_match(&path) {
            Ok(ConvertablePath::Windows(WindowsPath::new(path)))
        } else {
            Err("Invalid path format".to_string())
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            ConvertablePath::Windows(p) => p.path.clone(),
            ConvertablePath::Unix(p) => p.path.clone(),
            ConvertablePath::WSL(p) => p.path.clone(),
        }
    }

    pub fn path_type(&self) -> PathType {
        match self {
            ConvertablePath::Windows(_) => PathType::Windows,
            ConvertablePath::Unix(_) => PathType::Unix,
            ConvertablePath::WSL(_) => PathType::WSL,
        }
    }

    pub fn previous(&self) -> ConvertablePath {
        match self {
            ConvertablePath::Windows(p) => ConvertablePath::WSL(p.to_wsl()),
            ConvertablePath::Unix(p) => ConvertablePath::Windows(p.to_windows()),
            ConvertablePath::WSL(p) => ConvertablePath::Unix(p.to_unix()),
        }
    }

    pub fn next(&self) -> ConvertablePath {
        match self {
            ConvertablePath::Windows(p) => ConvertablePath::Unix(p.to_unix()),
            ConvertablePath::Unix(p) => ConvertablePath::WSL(p.to_wsl()),
            ConvertablePath::WSL(p) => ConvertablePath::Windows(p.to_windows()),
        }
    }
}

impl WindowsPath {
    pub fn new(path: String) -> Self {
        WindowsPath { path }
    }
}

impl UnixPath {
    pub fn new(path: String) -> Self {
        UnixPath { path }
    }
}

impl WslPath {
    pub fn new(path: String) -> Self {
        WslPath { path }
    }
}

pub trait PathConverter {
    fn to_windows(&self) -> WindowsPath;
    fn to_unix(&self) -> UnixPath;
    fn to_wsl(&self) -> WslPath;
}

impl PathConverter for WindowsPath {
    fn to_windows(&self) -> WindowsPath {
        self.clone()
    }

    fn to_unix(&self) -> UnixPath {
        let unix_path = self.path.replace('\\', "/");
        UnixPath::new(unix_path)
    }

    fn to_wsl(&self) -> WslPath {
        let wsl_path = format!(
            "/mnt/c/{}",
            self.path
                .replace('\\', "/")
                .trim_start_matches("C:")
                .trim_start_matches("c:")
        );
        WslPath::new(wsl_path)
    }
}

impl PathConverter for UnixPath {
    fn to_windows(&self) -> WindowsPath {
        let windows_path = self.path.replace('/', "\\");
        WindowsPath::new(windows_path)
    }

    fn to_unix(&self) -> UnixPath {
        self.clone()
    }

    fn to_wsl(&self) -> WslPath {
        let wsl_path = format!("/mnt/c/{}", self.path.trim_start_matches('/'));
        WslPath::new(wsl_path)
    }
}

impl PathConverter for WslPath {
    fn to_windows(&self) -> WindowsPath {
        let windows_path = self.path.trim_start_matches("/mnt/c/").replace('/', "\\");
        WindowsPath::new(format!("C:\\{}", windows_path))
    }

    fn to_unix(&self) -> UnixPath {
        let unix_path = self.path.trim_start_matches("/mnt/c");
        UnixPath::new(unix_path.to_string())
    }

    fn to_wsl(&self) -> WslPath {
        self.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_creation() {
        let windows_path =
            ConvertablePath::from_path(r"C:\Users\test\file.txt".to_string()).unwrap();
        assert_eq!(windows_path.path_type(), PathType::Windows);

        let unix_path = ConvertablePath::from_path("/home/user/file.txt".to_string()).unwrap();
        assert_eq!(unix_path.path_type(), PathType::Unix);

        let wsl_path =
            ConvertablePath::from_path("/mnt/c/Users/test/file.txt".to_string()).unwrap();
        assert_eq!(wsl_path.path_type(), PathType::WSL);
    }

    #[test]
    fn test_windows_regex() {
        let matching_paths = vec![
            r"C:\Users\test\file.txt",
            r"C:",
            r"d:\",
            r"\test\file.txt",
            r"test\file.txt",
            r"test\",
            r"\test",
        ];
        for path in matching_paths {
            println!("{}", path);
            assert!(WINDOWS_REGEX.is_match(path));
        }

        let non_matching_paths = vec!["Users", "C:Users", "C:/Users/test/file.txt"];
        for path in non_matching_paths {
            println!("{}", path);
            assert!(!WINDOWS_REGEX.is_match(path));
        }
    }

    #[test]
    fn test_unix_regex() {
        let matching_paths = vec!["/home/user/file.txt", "/home/user/", "home/user", "/"];
        for path in matching_paths {
            println!("{}", path);
            assert!(UNIX_REGEX.is_match(path));
        }

        let non_matching_paths = vec!["Users", "h\0me/user"];
        for path in non_matching_paths {
            println!("{}", path);
            assert!(!UNIX_REGEX.is_match(path));
        }
    }

    #[test]
    fn test_wsl_regex() {
        let matching_paths = vec![
            "/mnt/c/Users/test/file.txt",
            "/mnt/c/Users/",
            "/mnt/c/Users",
            "/mnt/D/",
            "/mnt/D",
        ];
        for path in matching_paths {
            println!("{}", path);
            assert!(WSL_REGEX.is_match(path));
        }

        let non_matching_paths = vec!["/mnt/drive/Users", "mnt/c/Users"];
        for path in non_matching_paths {
            println!("{}", path);
            assert!(!WSL_REGEX.is_match(path));
        }
    }
}
