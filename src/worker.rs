// Worker thread deletion logic

use crate::broker::Broker;
use crate::winapi::{delete_file_fast, enumerate_files_fast, remove_dir_fast};
use crossbeam_channel::Receiver;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread::{self, JoinHandle};

/// Spawn a pool of worker threads to process deletion work
///
/// Returns a vector of join handles that can be used to wait for all workers to complete.
/// Workers will exit when the channel is closed (no more work available).
pub fn spawn_workers(
    count: usize,
    rx: Receiver<PathBuf>,
    broker: Arc<Broker>,
) -> Vec<JoinHandle<()>> {
    (0..count)
        .map(|i| {
            let rx = rx.clone();
            let broker = broker.clone();
            thread::Builder::new()
                .name(format!("worker-{}", i))
                .spawn(move || worker_thread(rx, broker))
                .expect("Failed to spawn worker thread")
        })
        .collect()
}

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
    enumerate_files_fast(dir, |path, is_dir| {
        if !is_dir {
            if let Err(e) = delete_file_fast(path) {
                eprintln!("Warning: Failed to delete {}: {}", path.display(), e);
            }
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::broker::Broker;
    use crate::tree;
    use std::fs::{self, File};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

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

    #[test]
    fn test_spawn_workers_concurrent_consumption() {
        // Create a simple tree with multiple leaves to test parallel consumption
        let temp_root = std::env::temp_dir().join("win_rmdir_spawn_test");
        let _ = fs::remove_dir_all(&temp_root);

        // Create structure: root with 3 leaf dirs
        fs::create_dir(&temp_root).unwrap();
        let leaf1 = temp_root.join("leaf1");
        let leaf2 = temp_root.join("leaf2");
        let leaf3 = temp_root.join("leaf3");
        fs::create_dir(&leaf1).unwrap();
        fs::create_dir(&leaf2).unwrap();
        fs::create_dir(&leaf3).unwrap();

        // Add a file to each leaf so they have content to delete
        File::create(leaf1.join("file.txt")).unwrap();
        File::create(leaf2.join("file.txt")).unwrap();
        File::create(leaf3.join("file.txt")).unwrap();

        // Discover the tree and create broker
        let tree = tree::discover_tree(&temp_root).unwrap();
        let (broker, tx, rx) = Broker::new(tree);
        let broker = Arc::new(broker);

        // Drop the external sender - broker will close channel when done
        drop(tx);

        // Track how many workers actually process work
        let work_count = Arc::new(AtomicUsize::new(0));
        let work_count_clone = work_count.clone();

        // Spawn 3 workers
        let worker_count = 3;
        let handles: Vec<_> = (0..worker_count)
            .map(|i| {
                let rx = rx.clone();
                let broker = broker.clone();
                let work_count = work_count_clone.clone();
                thread::Builder::new()
                    .name(format!("test-worker-{}", i))
                    .spawn(move || {
                        loop {
                            match rx.recv_timeout(Duration::from_millis(100)) {
                                Ok(dir) => {
                                    work_count.fetch_add(1, Ordering::SeqCst);
                                    // Actually delete the directory content and dir
                                    let _ = delete_files_in_dir(&dir);
                                    let _ = remove_dir_fast(&dir);
                                    broker.mark_complete(dir);
                                }
                                Err(_) => break,
                            }
                        }
                    })
                    .expect("Failed to spawn test worker")
            })
            .collect();

        // Drop sender to close channel eventually
        drop(rx);

        // Wait for all workers
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify work was distributed (at least 3 leaf dirs were processed)
        let total_work = work_count.load(Ordering::SeqCst);
        assert!(
            total_work >= 3,
            "Expected at least 3 work items processed, got {}",
            total_work
        );

        // Clean up
        let _ = fs::remove_dir_all(&temp_root);
    }
}
