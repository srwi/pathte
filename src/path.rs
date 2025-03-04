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
        if WindowsPath::is_windows_path(&path) {
            Ok(WindowsPath { path })
        } else {
            Err("The given path is not a Windows path.".to_string())
        }
    }

    pub fn is_windows_path(path: &str) -> bool {
        if path.contains("\n") {
            return false;
        }

        WINDOWS_REGEX.is_match(path)
    }
}

impl UnixPath {
    pub fn new(path: String) -> Result<Self, String> {
        if UnixPath::is_unix_path(&path) {
            Ok(UnixPath { path })
        } else {
            Err("The given path is not a Unix path.".to_string())
        }
    }

    fn is_unix_path(path: &str) -> bool {
        if path.contains("//") || path.contains("\n") {
            return false;
        }

        UNIX_REGEX.is_match(path)
    }
}

impl WslPath {
    pub fn new(path: String) -> Result<Self, String> {
        if WslPath::is_wsl_path(&path) {
            Ok(WslPath { path })
        } else {
            Err("The given path is not a WSL path.".to_string())
        }
    }

    fn is_wsl_path(path: &str) -> bool {
        if path.contains("//") || path.contains("\n") {
            return false;
        }

        WSL_REGEX.is_match(path)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_creation() {
        let windows_path = WindowsPath::new(r"C:\Users\test\file.txt".to_string()).unwrap();
        assert_eq!(windows_path.get_type(), PathType::Windows);

        let unix_path = UnixPath::new("/home/user/file.txt".to_string()).unwrap();
        assert_eq!(unix_path.get_type(), PathType::Unix);

        let wsl_path = WslPath::new("/mnt/c/Users/test/file.txt".to_string()).unwrap();
        assert_eq!(wsl_path.get_type(), PathType::Wsl);
    }

    #[test]
    fn test_windows_matching() {
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
            assert!(WindowsPath::new(path.to_string()).is_ok());
        }

        let non_matching_paths = vec!["Users", "C:Users", "C:/Users/test/file.txt", "multi\nline"];
        for path in non_matching_paths {
            assert!(WindowsPath::new(path.to_string()).is_err());
        }
    }

    #[test]
    fn test_unix_matching() {
        let matching_paths = vec![
            "/home/user/file.txt",
            "/home/user/",
            "home/user",
            "/",
            "C:/",
        ];
        for path in matching_paths {
            assert!(UnixPath::new(path.to_string()).is_ok());
        }

        let non_matching_paths = vec!["Users", "h\0me/user", "// Comment", "multi\nline"];
        for path in non_matching_paths {
            assert!(UnixPath::new(path.to_string()).is_err());
        }
    }

    #[test]
    fn test_wsl_matching() {
        let matching_paths = vec![
            "/mnt/c/Users/test/file.txt",
            "/mnt/c/Users/",
            "/mnt/c/Users",
            "/mnt/D/",
            "/mnt/D",
        ];
        for path in matching_paths {
            assert!(WslPath::new(path.to_string()).is_ok());
        }

        let non_matching_paths = vec![
            "/mnt/drive/Users",
            "mnt/c/Users",
            "// Comment",
            "multi\nline",
        ];
        for path in non_matching_paths {
            assert!(WslPath::new(path.to_string()).is_err());
        }
    }

    #[test]
    fn test_windows_to_unix_conversion() {
        let pairs = vec![
            (r"C:\Users\test\file.txt", "C:/Users/test/file.txt"),
            (r"c:\", "c:/"),
            (r"\Users\test\", "/Users/test/"),
        ];

        for (input, expected) in pairs {
            let windows_path = WindowsPath::new(input.to_string()).unwrap();
            let unix_path = windows_path.to_unix().unwrap();
            assert_eq!(unix_path.as_string(), expected);
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
            let unix_path = UnixPath::new(input.to_string()).unwrap();
            let windows_path = unix_path.to_windows().unwrap();
            assert_eq!(windows_path.as_string(), expected);
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
            let wsl_path = WslPath::new(input.to_string()).unwrap();
            let windows_path = wsl_path.to_windows().unwrap();
            assert_eq!(windows_path.as_string(), expected);
        }
    }

    #[test]
    fn test_windows_to_wsl_conversion() {
        let pairs = vec![
            (r"c:\Users\test\file.txt", "/mnt/c/Users/test/file.txt"),
            (r"C:\", "/mnt/c/"),
            (r"D:", "/mnt/d"),
        ];

        for (input, expected) in pairs {
            let windows_path = WindowsPath::new(input.to_string()).unwrap();
            let wsl_path = windows_path.to_wsl().unwrap();
            assert_eq!(wsl_path.as_string(), expected);
        }
    }

    #[test]
    fn test_unix_to_wsl_conversion() {
        // TODO: This functionality is currently not supported

        // let pairs = vec![("C:/", "/mnt/c/"), ("d:/", "/mnt/d/")];

        // for (input, expected) in pairs {
        //     let unix_path = UnixPath::new(input.to_string()).unwrap();
        //     let wsl_path = unix_path.to_wsl().unwrap();
        //     assert_eq!(wsl_path.as_string(), expected);
        // }
    }

    #[test]
    fn test_wsl_to_unix_conversion() {
        let pairs = vec![
            ("/mnt/c/Users/test/file.txt", "/mnt/c/Users/test/file.txt"),
            ("/mnt/c/Users/", "/mnt/c/Users/"),
            ("/mnt/D/", "/mnt/D/"),
            ("/mnt/D", "/mnt/D"),
        ];

        for (input, expected) in pairs {
            let wsl_path = WslPath::new(input.to_string()).unwrap();
            let unix_path = wsl_path.to_unix().unwrap();
            assert_eq!(unix_path.as_string(), expected);
        }
    }
}
