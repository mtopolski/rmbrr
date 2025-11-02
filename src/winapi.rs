// Windows API wrappers for fast file/directory deletion

use std::io;
use std::path::Path;

#[cfg(windows)]
use windows::core::PCWSTR;
#[cfg(windows)]
use windows::Win32::Foundation::{CloseHandle, ERROR_ACCESS_DENIED, ERROR_FILE_NOT_FOUND, ERROR_SHARING_VIOLATION, HANDLE};
#[cfg(windows)]
use windows::Win32::Storage::FileSystem::{
    CreateFileW, DeleteFileW, RemoveDirectoryW, SetFileInformationByHandle,
    FILE_ACCESS_FLAGS, FILE_ATTRIBUTE_NORMAL, FILE_DISPOSITION_INFO,
    FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    FileDispositionInfo, OPEN_EXISTING, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
};

#[cfg(windows)]
fn path_to_wide(path: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(windows)]
pub fn delete_file_fast(path: &Path) -> io::Result<()> {
    let wide_path = path_to_wide(path);

    unsafe {
        // Try SetFileInformationByHandle approach for potential speed benefit
        let handle = CreateFileW(
            PCWSTR(wide_path.as_ptr()),
            FILE_ACCESS_FLAGS(0x10000), // DELETE access
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OPEN_REPARSE_POINT,
            HANDLE::default(),
        );

        if let Ok(h) = handle {
            let mut disposition = FILE_DISPOSITION_INFO {
                DeleteFile: true.into(),
            };

            let result = SetFileInformationByHandle(
                h,
                FileDispositionInfo,
                &mut disposition as *mut _ as *mut _,
                std::mem::size_of::<FILE_DISPOSITION_INFO>() as u32,
            );

            CloseHandle(h).ok();

            if result.is_ok() {
                return Ok(());
            }
        }

        // Fallback to DeleteFileW
        DeleteFileW(PCWSTR(wide_path.as_ptr()))
            .map_err(|e| {
                let code = e.code().0 as u32;
                match code {
                    x if x == ERROR_FILE_NOT_FOUND.0 => {
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
            })
    }
}

#[cfg(windows)]
pub fn remove_dir_fast(path: &Path) -> io::Result<()> {
    let wide_path = path_to_wide(path);

    unsafe {
        RemoveDirectoryW(PCWSTR(wide_path.as_ptr()))
            .map_err(|e| {
                let code = e.code().0 as u32;
                match code {
                    x if x == ERROR_FILE_NOT_FOUND.0 => {
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

#[cfg(not(windows))]
pub fn delete_file_fast(path: &Path) -> io::Result<()> {
    std::fs::remove_file(path)
}

#[cfg(not(windows))]
pub fn remove_dir_fast(path: &Path) -> io::Result<()> {
    std::fs::remove_dir(path)
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
