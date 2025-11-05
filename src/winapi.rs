// Platform-specific file/directory deletion implementations

use std::io;
use std::path::Path;

#[cfg(windows)]
use windows::core::PCWSTR;
#[cfg(windows)]
use windows::Wdk::Storage::FileSystem::{
    FILE_DISPOSITION_DELETE, FILE_DISPOSITION_IGNORE_READONLY_ATTRIBUTE,
    FILE_DISPOSITION_INFORMATION_EX, FILE_DISPOSITION_INFORMATION_EX_FLAGS,
    FILE_DISPOSITION_POSIX_SEMANTICS,
};
#[cfg(windows)]
use windows::Win32::Foundation::{CloseHandle, HANDLE};
#[cfg(windows)]
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FileDispositionInfoEx, FindClose, FindFirstFileExW, FindNextFileW,
    SetFileInformationByHandle, DELETE, FILE_ATTRIBUTE_DIRECTORY, FILE_FLAG_BACKUP_SEMANTICS,
    FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    FINDEX_INFO_LEVELS, FINDEX_SEARCH_OPS, FIND_FIRST_EX_FLAGS, OPEN_EXISTING, WIN32_FIND_DATAW,
};

#[cfg(windows)]
fn path_to_wide(path: &Path) -> Vec<u16> {
    let path_str = path.to_string_lossy();
    let prefixed = if path.is_absolute() && !path_str.starts_with(r"\\?\") {
        format!(r"\\?\{}", path.display())
    } else {
        path_str.to_string()
    };

    prefixed.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Delete file using POSIX semantics (immediate namespace removal)
/// Requires Windows 10 1607+ with NTFS
#[cfg(windows)]
pub fn delete_file(path: &Path) -> io::Result<()> {
    let wide_path = path_to_wide(path);
    unsafe { posix_delete_file(&wide_path) }
}

/// Delete directory using POSIX semantics (immediate namespace removal)
/// Requires Windows 10 1607+ with NTFS
#[cfg(windows)]
pub fn remove_dir(path: &Path) -> io::Result<()> {
    let wide_path = path_to_wide(path);
    unsafe { posix_delete_dir(&wide_path) }
}

#[cfg(windows)]
unsafe fn posix_delete_file(wide_path: &[u16]) -> io::Result<()> {
    let handle = CreateFileW(
        PCWSTR(wide_path.as_ptr()),
        DELETE.0,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        None,
        OPEN_EXISTING,
        FILE_FLAG_OPEN_REPARSE_POINT,
        HANDLE::default(),
    )
    .map_err(|e| io::Error::from_raw_os_error((e.code().0 & 0xFFFF) as i32))?;

    let mut info = FILE_DISPOSITION_INFORMATION_EX {
        Flags: FILE_DISPOSITION_INFORMATION_EX_FLAGS(
            FILE_DISPOSITION_DELETE.0
                | FILE_DISPOSITION_POSIX_SEMANTICS.0
                | FILE_DISPOSITION_IGNORE_READONLY_ATTRIBUTE.0,
        ),
    };

    let result = SetFileInformationByHandle(
        handle,
        FileDispositionInfoEx,
        &mut info as *mut _ as *mut _,
        std::mem::size_of::<FILE_DISPOSITION_INFORMATION_EX>() as u32,
    );

    CloseHandle(handle).ok();

    result.map_err(|e| {
        let code = (e.code().0 & 0xFFFF) as u32;
        io::Error::from_raw_os_error(code as i32)
    })
}

#[cfg(windows)]
unsafe fn posix_delete_dir(wide_path: &[u16]) -> io::Result<()> {
    let handle = CreateFileW(
        PCWSTR(wide_path.as_ptr()),
        DELETE.0,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        None,
        OPEN_EXISTING,
        FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
        HANDLE::default(),
    )
    .map_err(|e| io::Error::from_raw_os_error((e.code().0 & 0xFFFF) as i32))?;

    let mut info = FILE_DISPOSITION_INFORMATION_EX {
        Flags: FILE_DISPOSITION_INFORMATION_EX_FLAGS(
            FILE_DISPOSITION_DELETE.0 | FILE_DISPOSITION_POSIX_SEMANTICS.0,
        ),
    };

    let result = SetFileInformationByHandle(
        handle,
        FileDispositionInfoEx,
        &mut info as *mut _ as *mut _,
        std::mem::size_of::<FILE_DISPOSITION_INFORMATION_EX>() as u32,
    );

    CloseHandle(handle).ok();

    result.map_err(|e| {
        let code = (e.code().0 & 0xFFFF) as u32;
        io::Error::from_raw_os_error(code as i32)
    })
}

// Unix implementations - just use standard library
#[cfg(not(windows))]
pub fn delete_file(path: &Path) -> io::Result<()> {
    std::fs::remove_file(path)
}

#[cfg(not(windows))]
pub fn remove_dir(path: &Path) -> io::Result<()> {
    std::fs::remove_dir(path)
}

/// Enumerate files in a directory using direct Windows API
#[cfg(windows)]
pub fn enumerate_files<F>(dir: &Path, mut callback: F) -> io::Result<()>
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
            let name_len = find_data
                .cFileName
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(find_data.cFileName.len());
            let filename = String::from_utf16_lossy(&find_data.cFileName[..name_len]);

            if filename != "." && filename != ".." {
                let is_dir = (find_data.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY.0) != 0;
                let full_path = dir.join(&filename);
                callback(&full_path, is_dir)?;
            }

            if FindNextFileW(handle, &mut find_data).is_err() {
                break;
            }
        }

        let _ = FindClose(handle);
    }

    Ok(())
}

/// Enumerate files in a directory using standard library (Unix)
#[cfg(not(windows))]
pub fn enumerate_files<F>(dir: &Path, mut callback: F) -> io::Result<()>
where
    F: FnMut(&Path, bool) -> io::Result<()>,
{
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let is_dir = entry.file_type()?.is_dir();
        callback(&path, is_dir)?;
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
        delete_file(&test_file).unwrap();

        assert!(!test_file.exists());
    }

    #[test]
    fn test_delete_nonexistent_file() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("nonexistent_file_12345.txt");

        let result = delete_file(&test_file);
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
        remove_dir(&test_dir).unwrap();

        assert!(!test_dir.exists());
    }

    #[test]
    fn test_remove_nonexistent_dir() {
        let temp_dir = std::env::temp_dir();
        let test_dir = temp_dir.join("nonexistent_dir_12345");

        let result = remove_dir(&test_dir);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::NotFound);
    }
}
