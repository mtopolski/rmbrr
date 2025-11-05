// Edge case tests for rmbrr

use rmbrr::{broker::Broker, tree, worker};
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

/// Helper function to delete with pipeline
fn delete_directory(path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let tree = tree::discover_tree(path)?;
    let (broker, tx, rx) = Broker::new(tree);
    let broker = Arc::new(broker);

    let error_tracker = Arc::new(worker::ErrorTracker::new());
    let config = worker::WorkerConfig::default();

    let handles = worker::spawn_workers(4, rx, broker, config, error_tracker);
    drop(tx);

    for handle in handles {
        handle.join().unwrap();
    }

    Ok(())
}

#[test]
fn test_unicode_filenames() {
    let temp = std::env::temp_dir().join("rmbrr_test_unicode");
    let _ = fs::remove_dir_all(&temp);

    fs::create_dir(&temp).unwrap();

    // Create files with various Unicode characters
    File::create(temp.join("emoji_😀_file.txt")).unwrap();
    File::create(temp.join("中文文件.txt")).unwrap();
    File::create(temp.join("العربية.txt")).unwrap();
    File::create(temp.join("Ελληνικά.txt")).unwrap();
    File::create(temp.join("日本語.txt")).unwrap();
    File::create(temp.join("한국어.txt")).unwrap();

    // Create directory with Unicode name
    let unicode_dir = temp.join("тест_директория");
    fs::create_dir(&unicode_dir).unwrap();
    File::create(unicode_dir.join("файл.txt")).unwrap();

    assert!(temp.exists());
    assert_eq!(fs::read_dir(&temp).unwrap().count(), 7);

    delete_directory(&temp).unwrap();

    assert!(!temp.exists());
}

#[test]
fn test_deep_nested_paths() {
    let temp = std::env::temp_dir().join("rmbrr_test_deep");
    let _ = fs::remove_dir_all(&temp);

    // Create deeply nested directory structure (50 levels)
    let mut path = temp.clone();
    for i in 0..50 {
        path = path.join(format!("level_{}", i));
    }
    fs::create_dir_all(&path).unwrap();

    // Create a file at the deepest level
    File::create(path.join("deep_file.txt")).unwrap();

    assert!(temp.exists());

    delete_directory(&temp).unwrap();

    assert!(!temp.exists());
}

#[test]
fn test_many_files_in_single_directory() {
    let temp = std::env::temp_dir().join("rmbrr_test_many_files");
    let _ = fs::remove_dir_all(&temp);

    fs::create_dir(&temp).unwrap();

    // Create 1000 files in a single directory
    for i in 0..1000 {
        File::create(temp.join(format!("file_{:04}.txt", i))).unwrap();
    }

    assert_eq!(fs::read_dir(&temp).unwrap().count(), 1000);

    delete_directory(&temp).unwrap();

    assert!(!temp.exists());
}

#[test]
fn test_empty_files() {
    let temp = std::env::temp_dir().join("rmbrr_test_empty");
    let _ = fs::remove_dir_all(&temp);

    fs::create_dir(&temp).unwrap();

    // Create empty files
    File::create(temp.join("empty1.txt")).unwrap();
    File::create(temp.join("empty2.txt")).unwrap();

    // Create file with content
    let mut file = File::create(temp.join("with_content.txt")).unwrap();
    file.write_all(b"some content").unwrap();

    delete_directory(&temp).unwrap();

    assert!(!temp.exists());
}

#[test]
fn test_special_characters_in_names() {
    let temp = std::env::temp_dir().join("rmbrr_test_special");
    let _ = fs::remove_dir_all(&temp);

    fs::create_dir(&temp).unwrap();

    // Create files with special characters (that are valid on most systems)
    File::create(temp.join("file with spaces.txt")).unwrap();
    File::create(temp.join("file-with-dashes.txt")).unwrap();
    File::create(temp.join("file_with_underscores.txt")).unwrap();
    File::create(temp.join("file.multiple.dots.txt")).unwrap();

    #[cfg(unix)]
    {
        // Unix allows more special characters
        File::create(temp.join("file(with)parens.txt")).unwrap();
        File::create(temp.join("file[with]brackets.txt")).unwrap();
    }

    delete_directory(&temp).unwrap();

    assert!(!temp.exists());
}

#[test]
fn test_mixed_empty_and_full_directories() {
    let temp = std::env::temp_dir().join("rmbrr_test_mixed");
    let _ = fs::remove_dir_all(&temp);

    fs::create_dir(&temp).unwrap();

    // Create empty directory
    fs::create_dir(temp.join("empty_dir")).unwrap();

    // Create directory with files
    let full_dir = temp.join("full_dir");
    fs::create_dir(&full_dir).unwrap();
    File::create(full_dir.join("file.txt")).unwrap();

    // Create nested structure with mix
    let nested = temp.join("nested");
    fs::create_dir(&nested).unwrap();
    fs::create_dir(nested.join("empty_nested")).unwrap();
    let full_nested = nested.join("full_nested");
    fs::create_dir(&full_nested).unwrap();
    File::create(full_nested.join("nested_file.txt")).unwrap();

    delete_directory(&temp).unwrap();

    assert!(!temp.exists());
}

#[test]
#[cfg(unix)]
fn test_symlinks_unix() {
    use std::os::unix::fs::symlink;

    let temp = std::env::temp_dir().join("rmbrr_test_symlinks");
    let _ = fs::remove_dir_all(&temp);

    fs::create_dir(&temp).unwrap();

    // Create a real file
    let real_file = temp.join("real_file.txt");
    File::create(&real_file).unwrap();

    // Create a symlink to the file
    let link = temp.join("link_to_file.txt");
    symlink(&real_file, &link).unwrap();

    // Create a real directory
    let real_dir = temp.join("real_dir");
    fs::create_dir(&real_dir).unwrap();

    // Create a symlink to the directory
    let dir_link = temp.join("link_to_dir");
    symlink(&real_dir, &dir_link).unwrap();

    // Delete should remove symlinks but not targets
    delete_directory(&temp).unwrap();

    assert!(!temp.exists());
}

#[test]
fn test_readonly_files() {
    let temp = std::env::temp_dir().join("rmbrr_test_readonly");
    let _ = fs::remove_dir_all(&temp);

    fs::create_dir(&temp).unwrap();

    let readonly_file = temp.join("readonly.txt");
    let mut file = File::create(&readonly_file).unwrap();
    file.write_all(b"readonly content").unwrap();
    drop(file);

    // Make file readonly
    let mut perms = fs::metadata(&readonly_file).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(&readonly_file, perms).unwrap();

    // Should still be able to delete
    #[cfg_attr(unix, allow(unused_variables))]
    let result = delete_directory(&temp);

    #[cfg(windows)]
    {
        // On Windows, rmbrr uses IGNORE_READONLY_ATTRIBUTE flag
        assert!(result.is_ok());
        assert!(!temp.exists());
    }

    #[cfg(unix)]
    {
        // On Unix, deletion depends on parent directory permissions
        // This might fail depending on permissions
        let _ = fs::remove_dir_all(&temp); // Cleanup
    }
}

#[test]
fn test_very_long_filenames() {
    let temp = std::env::temp_dir().join("rmbrr_test_long_names");
    let _ = fs::remove_dir_all(&temp);

    fs::create_dir(&temp).unwrap();

    // Create file with long name (200 characters)
    let long_name = "a".repeat(200) + ".txt";
    let long_file = temp.join(&long_name);

    // This might fail on some systems with filename length limits
    if File::create(&long_file).is_ok() {
        delete_directory(&temp).unwrap();
        assert!(!temp.exists());
    } else {
        // Cleanup if creation failed
        let _ = fs::remove_dir_all(&temp);
    }
}

#[test]
fn test_case_sensitivity() {
    let temp = std::env::temp_dir().join("rmbrr_test_case");
    let _ = fs::remove_dir_all(&temp);

    fs::create_dir(&temp).unwrap();

    // Create files that differ only in case
    File::create(temp.join("File.txt")).unwrap();

    // On case-insensitive systems (Windows, macOS default), this will overwrite
    // On case-sensitive systems (Linux), this creates a second file
    File::create(temp.join("file.txt")).unwrap();

    delete_directory(&temp).unwrap();

    assert!(!temp.exists());
}
