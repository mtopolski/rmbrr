// Integration tests for rmbrr

use rmbrr::{broker::Broker, tree, worker};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

/// Test helper: create a directory tree with specified structure
fn create_test_tree(base: &Path, depth: usize, dirs_per_level: usize, files_per_dir: usize) {
    fs::create_dir_all(base).unwrap();

    // Create files in current directory
    for i in 0..files_per_dir {
        File::create(base.join(format!("file_{}.txt", i))).unwrap();
    }

    // Recurse to create subdirectories
    if depth > 0 {
        for i in 0..dirs_per_level {
            let subdir = base.join(format!("dir_{}", i));
            create_test_tree(&subdir, depth - 1, dirs_per_level, files_per_dir);
        }
    }
}

/// Count total directories in a path
fn count_dirs(path: &Path) -> usize {
    if !path.is_dir() {
        return 0;
    }

    let mut count = 1; // Count self
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                count += count_dirs(&entry.path());
            }
        }
    }
    count
}

/// Count total files in a path
fn count_files(path: &Path) -> usize {
    if !path.is_dir() {
        return 0;
    }

    let mut count = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                count += 1;
            } else if path.is_dir() {
                count += count_files(&path);
            }
        }
    }
    count
}

/// Run the deletion pipeline on a directory
fn delete_with_pipeline(path: &PathBuf) {
    let tree = tree::discover_tree(path).unwrap();
    let (broker, tx, rx) = Broker::new(tree);
    let broker = Arc::new(broker);

    let worker_count = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);

    let error_tracker = Arc::new(worker::ErrorTracker::new());
    let config = worker::WorkerConfig::default();

    let handles = worker::spawn_workers(worker_count, rx, broker, config, error_tracker);
    drop(tx);

    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_large_tree_1000_files_100_dirs() {
    let temp = std::env::temp_dir().join("win_rmdir_large_test");
    let _ = fs::remove_dir_all(&temp);

    // Create tree: depth=2, 10 dirs per level, 10 files per dir
    // Structure: root + 10 children + 100 grandchildren = 111 dirs
    // Files: 111 dirs * 10 files = 1110 files
    create_test_tree(&temp, 2, 10, 10);

    let dir_count = count_dirs(&temp);
    let file_count = count_files(&temp);

    println!("Created {} dirs with {} files", dir_count, file_count);
    assert!(dir_count >= 100, "Should have at least 100 dirs");
    assert!(file_count >= 1000, "Should have at least 1000 files");

    let start = Instant::now();
    delete_with_pipeline(&temp);
    let elapsed = start.elapsed();

    println!(
        "Deleted {} dirs and {} files in {:?}",
        dir_count, file_count, elapsed
    );
    assert!(!temp.exists(), "Directory should be deleted");
}

#[test]
fn test_deep_nesting_50_levels() {
    let temp = std::env::temp_dir().join("win_rmdir_deep_test");
    let _ = fs::remove_dir_all(&temp);

    // Create 50 levels deep: each level has 1 subdir and 1 file
    create_test_tree(&temp, 50, 1, 1);

    let dir_count = count_dirs(&temp);
    println!("Created {} nested directories", dir_count);
    assert_eq!(dir_count, 51, "Should have 51 dirs (root + 50 levels)");

    let start = Instant::now();
    delete_with_pipeline(&temp);
    let elapsed = start.elapsed();

    println!("Deleted deep tree ({} levels) in {:?}", dir_count, elapsed);
    assert!(!temp.exists(), "Directory should be deleted");
}

#[test]
fn test_wide_tree_1000_siblings() {
    let temp = std::env::temp_dir().join("win_rmdir_wide_test");
    let _ = fs::remove_dir_all(&temp);

    // Create 1000 sibling directories at root level
    create_test_tree(&temp, 1, 1000, 1);

    let dir_count = count_dirs(&temp);
    println!("Created {} sibling directories", dir_count);
    assert_eq!(
        dir_count, 1001,
        "Should have 1001 dirs (root + 1000 siblings)"
    );

    let start = Instant::now();
    delete_with_pipeline(&temp);
    let elapsed = start.elapsed();

    println!(
        "Deleted wide tree ({} siblings) in {:?}",
        dir_count, elapsed
    );
    assert!(!temp.exists(), "Directory should be deleted");
}

#[test]
fn test_empty_directories() {
    let temp = std::env::temp_dir().join("win_rmdir_empty_test");
    let _ = fs::remove_dir_all(&temp);

    // Create empty directory structure
    fs::create_dir_all(temp.join("a/b/c")).unwrap();
    fs::create_dir_all(temp.join("d/e")).unwrap();
    fs::create_dir(&temp.join("f")).unwrap();

    assert!(temp.exists());

    let start = Instant::now();
    delete_with_pipeline(&temp);
    let elapsed = start.elapsed();

    println!("Deleted empty tree in {:?}", elapsed);
    assert!(!temp.exists(), "Directory should be deleted");
}

#[test]
fn test_directory_with_only_files() {
    let temp = std::env::temp_dir().join("win_rmdir_files_only_test");
    let _ = fs::remove_dir_all(&temp);

    // Create directory with only files, no subdirectories
    fs::create_dir(&temp).unwrap();
    for i in 0..100 {
        File::create(temp.join(format!("file_{}.txt", i))).unwrap();
    }

    let file_count = count_files(&temp);
    assert_eq!(file_count, 100, "Should have 100 files");

    let start = Instant::now();
    delete_with_pipeline(&temp);
    let elapsed = start.elapsed();

    println!(
        "Deleted directory with {} files in {:?}",
        file_count, elapsed
    );
    assert!(!temp.exists(), "Directory should be deleted");
}

#[test]
fn test_mixed_structure() {
    let temp = std::env::temp_dir().join("win_rmdir_mixed_test");
    let _ = fs::remove_dir_all(&temp);

    // Create mixed structure with varying depth and file counts
    fs::create_dir_all(&temp).unwrap();

    // Deep branch
    create_test_tree(&temp.join("deep"), 10, 1, 5);

    // Wide branch
    create_test_tree(&temp.join("wide"), 1, 50, 2);

    // Empty branch
    fs::create_dir_all(temp.join("empty/a/b/c")).unwrap();

    // Files-only branch
    fs::create_dir(temp.join("files")).unwrap();
    for i in 0..20 {
        File::create(temp.join("files").join(format!("f{}.txt", i))).unwrap();
    }

    assert!(temp.exists());

    let start = Instant::now();
    delete_with_pipeline(&temp);
    let elapsed = start.elapsed();

    println!("Deleted mixed structure in {:?}", elapsed);
    assert!(!temp.exists(), "Directory should be deleted");
}
