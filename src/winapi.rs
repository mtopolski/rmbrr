// Windows API wrappers for fast file/directory deletion

use std::io;
use std::path::Path;

#[cfg(windows)]
use windows::core::PCWSTR;
#[cfg(windows)]
use windows::Win32::Foundation::{ERROR_ACCESS_DENIED, ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND, ERROR_SHARING_VIOLATION};
#[cfg(windows)]
use windows::Win32::Storage::FileSystem::{
    DeleteFileW, RemoveDirectoryW,
    SetFileAttributesW, FindFirstFileExW, FindNextFileW, FindClose,
    FILE_ATTRIBUTE_NORMAL,
    WIN32_FIND_DATAW, FILE_ATTRIBUTE_DIRECTORY,
    FINDEX_INFO_LEVELS, FINDEX_SEARCH_OPS, FIND_FIRST_EX_FLAGS,
};

#[cfg(windows)]
fn path_to_wide(path: &Path) -> Vec<u16> {
    let path_str = path.to_string_lossy();
    let prefixed = if path.is_absolute() && !path_str.starts_with(r"\\?\") {
        format!(r"\\?\{}", path.display())
    } else {
        path_str.to_string()
    };

    prefixed
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(windows)]
pub fn delete_file_fast(path: &Path) -> io::Result<()> {
    let wide_path = path_to_wide(path);

    unsafe {
        // Try delete first
        let result = DeleteFileW(PCWSTR(wide_path.as_ptr()));

        // If access denied, try clearing read-only flag and retry
        if let Err(e) = result {
            let code = (e.code().0 & 0xFFFF) as u32;
            if code == ERROR_ACCESS_DENIED.0 {
                // Clear read-only flag and retry
                if SetFileAttributesW(PCWSTR(wide_path.as_ptr()), FILE_ATTRIBUTE_NORMAL).is_ok() {
                    return DeleteFileW(PCWSTR(wide_path.as_ptr()))
                        .map_err(|e| {
                            let code = (e.code().0 & 0xFFFF) as u32;
                            match code {
                                x if x == ERROR_FILE_NOT_FOUND.0 || x == ERROR_PATH_NOT_FOUND.0 => {
                                    io::Error::new(io::ErrorKind::NotFound, "File not found")
                                }
                                x if x == ERROR_ACCESS_DENIED.0 => {
                                    io::Error::new(io::ErrorKind::PermissionDenied, "Access denied")
                                }
                                x if x == ERROR_SHARING_VIOLATION.0 => {
                                    io::Error::new(io::ErrorKind::PermissionDenied, "File in use")
                                }
                                _ => io::Error::from_raw_os_error(code as i32),
                            }
                        });
                }
            }

            // Map other errors
            match code {
                x if x == ERROR_FILE_NOT_FOUND.0 || x == ERROR_PATH_NOT_FOUND.0 => {
                    Err(io::Error::new(io::ErrorKind::NotFound, "File not found"))
                }
                x if x == ERROR_ACCESS_DENIED.0 => {
                    Err(io::Error::new(io::ErrorKind::PermissionDenied, "Access denied"))
                }
                x if x == ERROR_SHARING_VIOLATION.0 => {
                    Err(io::Error::new(io::ErrorKind::PermissionDenied, "File in use"))
                }
                _ => Err(io::Error::from_raw_os_error(code as i32)),
            }
        } else {
            Ok(())
        }
    }
}

#[cfg(windows)]
pub fn remove_dir_fast(path: &Path) -> io::Result<()> {
    let wide_path = path_to_wide(path);

    unsafe {
        RemoveDirectoryW(PCWSTR(wide_path.as_ptr()))
            .map_err(|e| {
                let code = (e.code().0 & 0xFFFF) as u32;
                match code {
                    x if x == ERROR_FILE_NOT_FOUND.0 || x == ERROR_PATH_NOT_FOUND.0 => {
                        io::Error::new(io::ErrorKind::NotFound, "Directory not found")
                    }
                    x if x == ERROR_ACCESS_DENIED.0 => {
                        io::Error::new(io::ErrorKind::PermissionDenied, "Access denied")
                    }
                    _ => io::Error::from_raw_os_error(code as i32),
                }
            })
    }
}


/// Enumerate files in a directory using direct Windows API
#[cfg(windows)]
pub fn enumerate_files_fast<F>(dir: &Path, mut callback: F) -> io::Result<()>
where
    F: FnMut(&Path, bool) -> io::Result<()>,
{
    let search_path = dir.join("*");
    let wide_path = path_to_wide(&search_path);

    unsafe {
        let mut find_data: WIN32_FIND_DATAW = std::mem::zeroed();
        let handle = match FindFirstFileExW(
            PCWSTR(wide_path.as_ptr()),
            FINDEX_INFO_LEVELS(1),
            &mut find_data as *mut _ as *mut _,
            FINDEX_SEARCH_OPS(0),
            None,
            FIND_FIRST_EX_FLAGS(0),
        ) {
            Ok(h) => h,
            Err(_) => return Err(io::Error::last_os_error()),
        };

        loop {
            // Convert filename from wide string
            let name_len = find_data.cFileName.iter().position(|&c| c == 0).unwrap_or(find_data.cFileName.len());
            let filename = String::from_utf16_lossy(&find_data.cFileName[..name_len]);

            // Skip . and ..
            if filename != "." && filename != ".." {
                let is_dir = (find_data.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY.0) != 0;
                let full_path = dir.join(&filename);
                callback(&full_path, is_dir)?;
            }

            // Get next file
            if FindNextFileW(handle, &mut find_data).is_err() {
                break;
            }
        }

        let _ = FindClose(handle);
    }

    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_delete_file() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("win_rmdir_test_file.txt");

        // Create test file
        let mut file = File::create(&test_file).unwrap();
        file.write_all(b"test").unwrap();
        drop(file);

        assert!(test_file.exists());

        // Delete it
        delete_file_fast(&test_file).unwrap();

        assert!(!test_file.exists());
    }

    #[test]
    fn test_delete_nonexistent_file() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("nonexistent_file_12345.txt");

        let result = delete_file_fast(&test_file);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn test_remove_dir() {
        let temp_dir = std::env::temp_dir();
        let test_dir = temp_dir.join("win_rmdir_test_dir");

        // Create test directory
        std::fs::create_dir(&test_dir).unwrap();
        assert!(test_dir.exists());

        // Delete it
        remove_dir_fast(&test_dir).unwrap();

        assert!(!test_dir.exists());
    }

    #[test]
    fn test_remove_nonexistent_dir() {
        let temp_dir = std::env::temp_dir();
        let test_dir = temp_dir.join("nonexistent_dir_12345");

        let result = remove_dir_fast(&test_dir);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::NotFound);
    }
}
