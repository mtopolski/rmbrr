//! Safety checks and validation for rmbrr

use std::env;
use std::path::{Path, PathBuf};

/// Check if a path is a protected system directory
pub fn is_system_directory(path: &Path) -> bool {
    let canonical = path.canonicalize().ok();
    let path_str = path.to_string_lossy();
    let canonical_str = canonical.as_ref().map(|p| p.to_string_lossy());

    // Check common system directories
    #[cfg(windows)]
    {
        let protected_windows = [
            "C:\\Windows",
            "C:\\Windows\\System32",
            "C:\\Program Files",
            "C:\\Program Files (x86)",
            "C:\\ProgramData",
            "C:\\",
            "C:\\Users",
        ];

        for protected in &protected_windows {
            if path_str.eq_ignore_ascii_case(protected) {
                return true;
            }
            if let Some(ref canonical) = canonical_str {
                if canonical.eq_ignore_ascii_case(protected) {
                    return true;
                }
            }
        }

        // Check if it's a drive root (C:\, D:\, etc.)
        if path_str.len() <= 3 && path_str.ends_with(":\\") {
            return true;
        }
    }

    #[cfg(unix)]
    {
        let protected_unix = [
            "/", "/bin", "/boot", "/dev", "/etc", "/lib", "/lib64", "/proc", "/root", "/sbin",
            "/sys", "/usr", "/var",
        ];

        for protected in &protected_unix {
            if path_str == *protected {
                return true;
            }
            if let Some(ref canonical) = canonical_str {
                if canonical.as_ref() == *protected {
                    return true;
                }
            }
        }
    }

    // Check if it's the user's home directory
    if let Ok(home) = env::var("HOME") {
        let home_path = PathBuf::from(home);
        if let (Ok(p1), Ok(p2)) = (path.canonicalize(), home_path.canonicalize()) {
            if p1 == p2 {
                return true;
            }
        }
    }

    // Windows USERPROFILE
    #[cfg(windows)]
    {
        if let Ok(userprofile) = env::var("USERPROFILE") {
            let user_path = PathBuf::from(userprofile);
            if let (Ok(p1), Ok(p2)) = (path.canonicalize(), user_path.canonicalize()) {
                if p1 == p2 {
                    return true;
                }
            }
        }
    }

    false
}

/// Check if path is within current working directory
pub fn is_in_current_directory(path: &Path) -> bool {
    if let Ok(cwd) = env::current_dir() {
        if let (Ok(p1), Ok(p2)) = (path.canonicalize(), cwd.canonicalize()) {
            // Check if path is the CWD itself or a parent of CWD
            return p1 == p2 || cwd.starts_with(&p1);
        }
    }
    false
}

/// Get a human-readable description of why a path might be dangerous
pub fn get_danger_reason(path: &Path) -> Option<String> {
    if is_system_directory(path) {
        return Some(format!(
            "'{}' is a system directory - deleting it could break your system",
            path.display()
        ));
    }

    if is_in_current_directory(path) {
        return Some(format!(
            "'{}' contains or is your current working directory",
            path.display()
        ));
    }

    None
}

/// Safety check result
#[derive(Debug)]
pub enum SafetyCheck {
    Safe,
    Dangerous { reason: String, can_override: bool },
}

/// Perform comprehensive safety checks on a path
pub fn check_path_safety(path: &Path) -> SafetyCheck {
    if let Some(reason) = get_danger_reason(path) {
        SafetyCheck::Dangerous {
            reason,
            can_override: !is_system_directory(path), // System dirs cannot be overridden
        }
    } else {
        SafetyCheck::Safe
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_directory_detection() {
        #[cfg(unix)]
        {
            assert!(is_system_directory(Path::new("/")));
            assert!(is_system_directory(Path::new("/bin")));
            assert!(is_system_directory(Path::new("/usr")));
            assert!(!is_system_directory(Path::new("/tmp/test")));
        }

        #[cfg(windows)]
        {
            assert!(is_system_directory(Path::new("C:\\")));
            assert!(is_system_directory(Path::new("C:\\Windows")));
            assert!(!is_system_directory(Path::new("C:\\temp\\test")));
        }
    }

    #[test]
    fn test_temp_directory_safe() {
        let temp = std::env::temp_dir();
        assert!(!is_system_directory(&temp));
    }

    #[test]
    fn test_safety_check() {
        let temp = std::env::temp_dir().join("test_safety");

        match check_path_safety(&temp) {
            SafetyCheck::Safe => {
                // Expected - temp dir is safe
            }
            SafetyCheck::Dangerous { .. } => {
                panic!("Temp directory should be safe");
            }
        }
    }
}
