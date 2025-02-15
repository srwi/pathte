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
    static ref PROTOCOL_REGEX: Regex = Regex::new(r"(http|https|ftp|sftp|file):$").unwrap();
}

impl ConvertablePath {
    pub fn from_path(path: String) -> Result<Self, String> {
        if path.contains('\n') || PROTOCOL_REGEX.is_match(&path) {
            return Err("Invalid path format".to_string());
        }

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
        let drive_regex = Regex::new(r"^([A-Za-z]):").unwrap();
        let wsl_path = drive_regex
            .replace(&self.path, |captures: &regex::Captures| {
                format!("/mnt/{}", &captures[1].to_lowercase())
            })
            .replace("\\", "/");
        WslPath::new(wsl_path)
    }
}

impl PathConverter for UnixPath {
    fn to_windows(&self) -> WindowsPath {
        let windows_path = self.path.replace("/", "\\");
        WindowsPath::new(windows_path)
    }

    fn to_unix(&self) -> UnixPath {
        self.clone()
    }

    fn to_wsl(&self) -> WslPath {
        let drive_regex = Regex::new(r"^([A-Za-z]):").unwrap();
        if drive_regex.is_match(&self.path) {
            let windows_path = self.to_windows();
            windows_path.to_wsl()
        } else {
            WslPath::new(self.path.clone())
        }
    }
}

impl PathConverter for WslPath {
    fn to_windows(&self) -> WindowsPath {
        let drive_regex = Regex::new(r"^/mnt/([A-Za-z])").unwrap();
        let windows_path = drive_regex
            .replace(&self.path, |captures: &regex::Captures| {
                format!("{}:", &captures[1].to_uppercase())
            })
            .replace('/', "\\");
        WindowsPath::new(windows_path)
    }

    fn to_unix(&self) -> UnixPath {
        UnixPath::new(self.path.clone())
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

    #[test]
    fn test_windows_to_unix_conversion() {
        let pairs = vec![
            (r"C:\Users\test\file.txt", "C:/Users/test/file.txt"),
            (r"c:\", "c:/"),
            (r"d:", "d:"),
            (r"\Users\test\", "/Users/test/"),
        ];

        for (input, expected) in pairs {
            let windows_path = WindowsPath::new(input.to_string());
            let unix_path = windows_path.to_unix();
            assert_eq!(unix_path.path, expected);
        }
    }

    #[test]
    fn test_unix_to_windows_conversion() {
        let pairs = vec![
            ("/home/user/file.txt", r"\home\user\file.txt"),
            ("home/user/", r"home\user\"),
            ("/", r"\"),
            ("C:/", r"C:\"),
            ("d:/", r"d:\"),
        ];

        for (input, expected) in pairs {
            let unix_path = UnixPath::new(input.to_string());
            let windows_path = unix_path.to_windows();
            assert_eq!(windows_path.path, expected);
        }
    }

    #[test]
    fn test_wsl_to_windows_conversion() {
        let pairs = vec![
            ("/mnt/c/Users/test/file.txt", r"C:\Users\test\file.txt"),
            ("/mnt/c/Users/", r"C:\Users\"),
            ("/mnt/D/", r"D:\"),
            ("/mnt/D", r"D:"),
        ];

        for (input, expected) in pairs {
            let wsl_path = WslPath::new(input.to_string());
            let windows_path = wsl_path.to_windows();
            assert_eq!(windows_path.path, expected);
        }
    }

    #[test]
    fn test_windows_to_wsl_conversion() {
        let pairs = vec![
            (r"c:\Users\test\file.txt", "/mnt/c/Users/test/file.txt"),
            (r"C:\", "/mnt/c/"),
            (r"D:", "/mnt/d"),
            (r"\Users\test\", "/Users/test/"),
        ];

        for (input, expected) in pairs {
            let windows_path = WindowsPath::new(input.to_string());
            let wsl_path = windows_path.to_wsl();
            assert_eq!(wsl_path.path, expected);
        }
    }
}
