// Worker thread deletion logic

use crate::broker::Broker;
use crate::winapi::{delete_file_fast, remove_dir_fast};
use crossbeam_channel::Receiver;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

pub fn worker_thread(rx: Receiver<PathBuf>, broker: Arc<Broker>) {
    loop {
        // Block waiting for work
        let dir = match rx.recv() {
            Ok(d) => d,
            Err(_) => break, // Channel closed, exit
        };

        // Delete all files in directory
        if let Err(e) = delete_files_in_dir(&dir) {
            eprintln!("Warning: Failed to delete files in {}: {}", dir.display(), e);
            // Continue anyway - try to delete directory
        }

        // Delete the directory itself
        if let Err(e) = remove_dir_fast(&dir) {
            eprintln!("Warning: Failed to remove {}: {}", dir.display(), e);
            // Don't notify broker on failure - this will stall parent deletion
            // In production, might want to track failures separately
            continue;
        }

        // Notify broker that this directory is done
        broker.mark_complete(dir);
    }
}

fn delete_files_in_dir(dir: &PathBuf) -> std::io::Result<()> {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => return Err(e),
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();

        let is_file = match entry.file_type() {
            Ok(ft) => ft.is_file() || ft.is_symlink(),
            Err(_) => continue,
        };

        if is_file {
            if let Err(e) = delete_file_fast(&path) {
                eprintln!("Warning: Failed to delete {}: {}", path.display(), e);
                // Continue with other files
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    #[test]
    fn test_delete_files_in_dir() {
        let temp = std::env::temp_dir().join("win_rmdir_delete_files_test");
        let _ = fs::remove_dir_all(&temp);

        fs::create_dir(&temp).unwrap();
        File::create(temp.join("a.txt")).unwrap();
        File::create(temp.join("b.txt")).unwrap();
        File::create(temp.join("c.txt")).unwrap();

        assert_eq!(fs::read_dir(&temp).unwrap().count(), 3);

        delete_files_in_dir(&temp).unwrap();

        // Files should be deleted, dir still exists
        assert_eq!(fs::read_dir(&temp).unwrap().count(), 0);
        assert!(temp.exists());

        fs::remove_dir(&temp).ok();
    }
}
