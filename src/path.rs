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

impl ConvertablePath {
    pub fn from_path(path: String) -> Result<Self, String> {
        if path.starts_with("/mnt/") {
            Ok(ConvertablePath::WSL(WslPath::new(path)))
        } else if path.contains('\\') || path.contains("C:") || path.contains("c:") {
            Ok(ConvertablePath::Windows(WindowsPath::new(path)))
        } else if path.starts_with('/') {
            Ok(ConvertablePath::Unix(UnixPath::new(path)))
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
        let unix_path = ConvertablePath::from_path("/home/user/file.txt".to_string()).unwrap();
        let wsl_path =
            ConvertablePath::from_path("/mnt/c/Users/test/file.txt".to_string()).unwrap();

        assert_eq!(windows_path.path_type(), PathType::Windows);
        assert_eq!(unix_path.path_type(), PathType::Unix);
        assert_eq!(wsl_path.path_type(), PathType::WSL);
    }

    #[test]
    fn test_path_conversion() {
        let windows_path =
            ConvertablePath::Windows(WindowsPath::new(r"C:\Users\test\file.txt".to_string()));
        let unix_path = windows_path.next();
        let wsl_path = unix_path.next();

        match &unix_path {
            ConvertablePath::Unix(p) => assert_eq!(p.path, "C:/Users/test/file.txt"),
            _ => panic!("Expected Unix path"),
        }

        match &wsl_path {
            ConvertablePath::WSL(p) => assert_eq!(p.path, "/mnt/c/Users/test/file.txt"),
            _ => panic!("Expected WSL path"),
        }
    }
}
