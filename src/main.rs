use clap::Parser;
use std::path::PathBuf;
use std::process;
use std::sync::Arc;
use std::time::Instant;
use win_rmdir_fast::{broker::Broker, tree, worker};

/// Hyper-optimized parallel directory deletion for Windows
#[derive(Parser, Debug)]
#[command(name = "win-rmdir-fast")]
#[command(version, about, long_about = None)]
struct Args {
    /// Target directory to delete
    path: PathBuf,

    /// Number of worker threads (default: logical CPU count)
    #[arg(short = 't', long)]
    threads: Option<usize>,

    /// Dry run - scan and plan but don't delete anything
    #[arg(short = 'n', long)]
    dry_run: bool,

    /// Silent mode - disable progress output for maximum performance
    #[arg(short = 's', long)]
    silent: bool,
}

fn main() {
    let args = Args::parse();

    // Validate path exists
    if !args.path.exists() {
        eprintln!("Error: Path does not exist: {}", args.path.display());
        process::exit(1);
    }

    if !args.path.is_dir() {
        eprintln!("Error: Path is not a directory: {}", args.path.display());
        process::exit(1);
    }

    if args.dry_run {
        println!("DRY RUN MODE - no files will be deleted");
    }

    // Determine worker count
    let worker_count = args.threads.unwrap_or_else(|| {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
    });

    if !args.silent {
        println!("Scanning directory tree: {}", args.path.display());
    }
    let start = Instant::now();

    // Phase 1: Discover tree
    let tree = match tree::discover_tree(&args.path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Error scanning directory: {}", e);
            process::exit(1);
        }
    };

    let scan_time = start.elapsed();
    if !args.silent {
        println!(
            "Found {} directories ({} initial leaves), {} files in {:.2?}",
            tree.dirs.len(),
            tree.leaves.len(),
            tree.file_count,
            scan_time
        );
    }

    if args.dry_run {
        println!("Dry run complete. Would delete {} directories and {} files.", tree.dirs.len(), tree.file_count);
        return;
    }

    // Phase 2: Initialize broker and spawn workers
    let (broker, tx, rx) = Broker::new(tree);
    let broker = Arc::new(broker);

    if !args.silent {
        println!("Spawning {} worker threads...", worker_count);
    }
    let handles = worker::spawn_workers(worker_count, rx, broker.clone());

    // Phase 3: Drop the sender to signal completion when all work is done
    drop(tx);

    // Wait for all workers to finish
    if !args.silent {
        println!("Deleting directories...");
    }
    let delete_start = Instant::now();

    // Progress monitoring thread
    let progress_handle = if !args.silent {
        let total = broker.total_dirs();
        let broker_clone = broker.clone();
        Some(std::thread::spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_millis(250));
                let completed = broker_clone.completed_count();
                if completed >= total {
                    break;
                }
                let pct = (completed as f64 / total as f64 * 100.0) as u32;
                print!("\rDeleting... {}% ({}/{} dirs)", pct, completed, total);
                use std::io::Write;
                std::io::stdout().flush().ok();
            }
        }))
    } else {
        None
    };

    for handle in handles {
        handle.join().expect("Worker thread panicked");
    }

    if let Some(handle) = progress_handle {
        handle.join().ok();
        let total = broker.total_dirs();
        println!("\rDeleting... 100% ({}/{} dirs) - Complete!", total, total);
    }

    let delete_time = delete_start.elapsed();
    let total_time = start.elapsed();

    println!("\nDeletion complete!");
    if !args.silent {
        println!("  Scan time:   {:.2?}", scan_time);
        println!("  Delete time: {:.2?}", delete_time);
        println!("  Total time:  {:.2?}", total_time);
    }
}
