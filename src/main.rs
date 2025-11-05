use clap::Parser;
use rmbrr::{broker::Broker, error::Error, safety, tree, worker};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::Arc;
use std::time::Instant;

/// Windows efficient rmdir with cross-platform compatibility
#[derive(Parser, Debug)]
#[command(name = "rmbrr")]
#[command(version)]
#[command(about = "Fast, parallel directory deletion with cross-platform support")]
#[command(
    long_about = "rmbrr (rm + brrr) is a high-performance directory deletion tool that uses \
parallel processing and platform-specific optimizations. On Windows, it uses POSIX semantics \
for immediate namespace removal. Benchmarks show 2-6x faster than alternatives like rimraf."
)]
#[command(after_help = "EXAMPLES:\n  \
  rmbrr ./node_modules              Delete a directory\n  \
  rmbrr -n ./build                  Dry run (preview what would be deleted)\n  \
  rmbrr -v ./dist                   Verbose mode (show all errors)\n  \
  rmbrr --stats ./target            Show detailed statistics\n  \
  rmbrr --confirm ./data            Ask for confirmation before deleting\n  \
  rmbrr ./dir1 ./dir2 ./dir3        Delete multiple directories\n\n\
For more information, visit: https://github.com/mtopolski/rmbrr")]
struct Args {
    /// Target directory(s) to delete
    #[arg(required = true)]
    paths: Vec<PathBuf>,

    /// Number of worker threads (default: logical CPU count)
    #[arg(short = 't', long)]
    threads: Option<usize>,

    /// Dry run - scan and plan but don't delete anything
    #[arg(short = 'n', long)]
    dry_run: bool,

    /// Silent mode - disable progress output for maximum performance
    #[arg(short = 's', long, alias = "no-progress")]
    silent: bool,

    /// Verbose error reporting - print all errors as they occur
    #[arg(short = 'v', long)]
    verbose: bool,

    /// Ignore errors and continue deletion (default behavior)
    #[arg(long, default_value_t = true)]
    ignore_errors: bool,

    /// Ask for confirmation before deleting
    #[arg(short = 'c', long)]
    confirm: bool,

    /// Show detailed statistics at the end
    #[arg(long)]
    stats: bool,

    /// Force deletion of dangerous paths (use with extreme caution)
    #[arg(long)]
    force: bool,
}

fn main() {
    let args = Args::parse();

    if let Err(e) = run(args) {
        eprintln!("Error: {}", e);
        process::exit(e.exit_code());
    }
}

fn run(args: Args) -> Result<(), Error> {
    let mut total_stats = DeletionStats::default();
    let mut all_failures = Vec::new();
    let mut failed_paths = Vec::new();

    for (i, path) in args.paths.iter().enumerate() {
        if args.paths.len() > 1 && !args.silent {
            println!(
                "\n[{}/{}] Processing: {}",
                i + 1,
                args.paths.len(),
                path.display()
            );
        }

        match process_single_path(path, &args) {
            Ok(stats) => {
                total_stats.merge(&stats);
            }
            Err(e) => {
                eprintln!("Failed to process {}: {}", path.display(), e);
                failed_paths.push(path.to_path_buf());
                if let Error::PartialFailure { errors, .. } = e {
                    all_failures.extend(errors);
                }
            }
        }
    }

    if args.paths.len() > 1 && !args.silent {
        print_summary(&total_stats, &all_failures, &failed_paths, &args);
    }

    if !failed_paths.is_empty() || !all_failures.is_empty() {
        Err(Error::PartialFailure {
            total: total_stats.total_items(),
            failed: all_failures.len() + failed_paths.len(),
            errors: all_failures,
        })
    } else {
        Ok(())
    }
}

#[derive(Default)]
struct DeletionStats {
    dirs_deleted: usize,
    files_deleted: usize,
    total_scan_time: std::time::Duration,
    total_delete_time: std::time::Duration,
}

impl DeletionStats {
    fn merge(&mut self, other: &DeletionStats) {
        self.dirs_deleted += other.dirs_deleted;
        self.files_deleted += other.files_deleted;
        self.total_scan_time += other.total_scan_time;
        self.total_delete_time += other.total_delete_time;
    }

    fn total_items(&self) -> usize {
        self.dirs_deleted + self.files_deleted
    }
}

fn print_summary(
    stats: &DeletionStats,
    failures: &[rmbrr::error::FailedItem],
    failed_paths: &[PathBuf],
    args: &Args,
) {
    println!("\n{}", "=".repeat(60));
    println!("SUMMARY");
    println!("{}", "=".repeat(60));
    println!("Paths processed: {}", args.paths.len());
    println!("Directories deleted: {}", stats.dirs_deleted);
    println!("Files deleted: {}", stats.files_deleted);
    if !failures.is_empty() {
        println!("Failed items: {}", failures.len());
    }
    if !failed_paths.is_empty() {
        println!("Failed paths: {}", failed_paths.len());
    }
    if args.stats {
        println!("\nTiming:");
        println!("  Total scan time:   {:.2?}", stats.total_scan_time);
        println!("  Total delete time: {:.2?}", stats.total_delete_time);
        println!(
            "  Total time:        {:.2?}",
            stats.total_scan_time + stats.total_delete_time
        );
    }
}

fn process_single_path(path: &Path, args: &Args) -> Result<DeletionStats, Error> {
    if !path.exists() {
        return Err(Error::InvalidPath {
            path: path.to_path_buf(),
            reason: "path does not exist".to_string(),
        });
    }

    if !path.is_dir() {
        return Err(Error::InvalidPath {
            path: path.to_path_buf(),
            reason: "not a directory".to_string(),
        });
    }

    match safety::check_path_safety(path) {
        safety::SafetyCheck::Safe => {}
        safety::SafetyCheck::Dangerous {
            reason,
            can_override,
        } => {
            if !args.force {
                eprintln!("\n⚠️  WARNING: Dangerous operation detected!");
                eprintln!("   {}", reason);
                eprintln!();

                if can_override {
                    eprintln!("   To proceed anyway, use the --force flag");
                    eprintln!("   Example: rmbrr --force {}", path.display());
                } else {
                    eprintln!("   This path cannot be deleted for safety reasons.");
                    eprintln!("   Deletion of system directories is not allowed.");
                }
                eprintln!();

                return Err(Error::InvalidPath {
                    path: path.to_path_buf(),
                    reason: "dangerous path - requires --force (if allowed)".to_string(),
                });
            } else if !can_override {
                eprintln!("\n⛔ ERROR: Cannot delete system directory");
                eprintln!("   {}", reason);
                eprintln!("   System directories cannot be deleted even with --force");
                eprintln!();

                return Err(Error::InvalidPath {
                    path: path.to_path_buf(),
                    reason: "system directory cannot be deleted".to_string(),
                });
            } else {
                if !args.silent {
                    eprintln!("\n⚠️  WARNING: Deleting dangerous path with --force");
                    eprintln!("   {}", reason);
                    eprintln!();
                }
            }
        }
    }

    if args.dry_run && !args.silent {
        println!("DRY RUN MODE - no files will be deleted");
    }

    let worker_count = args.threads.unwrap_or_else(|| {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
    });

    if !args.silent {
        println!("Scanning directory tree: {}", path.display());
    }
    let start = Instant::now();

    let tree = tree::discover_tree(path).map_err(|e| Error::io_with_path(path.to_path_buf(), e))?;

    let scan_time = start.elapsed();
    let dir_count = tree.dirs.len();
    let file_count = tree.file_count;

    if !args.silent {
        println!(
            "Found {} directories ({} initial leaves), {} files in {:.2?}",
            dir_count,
            tree.leaves.len(),
            file_count,
            scan_time
        );
    }

    if args.confirm && !args.dry_run {
        println!("\nAbout to delete:");
        println!("  {} directories", dir_count);
        println!("  {} files", file_count);
        println!("  Total: {} items", dir_count + file_count);
        println!("\nAre you sure? [y/N] ");

        use std::io::{self, BufRead};
        let stdin = io::stdin();
        let mut response = String::new();
        stdin.lock().read_line(&mut response).ok();

        let response = response.trim().to_lowercase();
        if response != "y" && response != "yes" {
            println!("Aborted.");
            return Ok(DeletionStats {
                dirs_deleted: 0,
                files_deleted: 0,
                total_scan_time: scan_time,
                total_delete_time: std::time::Duration::ZERO,
            });
        }
    }

    if args.dry_run {
        if !args.silent {
            println!("\n{}", "=".repeat(60));
            println!("DRY RUN RESULTS");
            println!("{}", "=".repeat(60));
            println!("\nWould delete:");
            println!("  {} directories", dir_count);
            println!("  {} files", file_count);
            println!("  {} total items", dir_count + file_count);

            println!("\nTo proceed with deletion:");
            println!("  rmbrr {}", path.display());
        }
        return Ok(DeletionStats {
            dirs_deleted: dir_count,
            files_deleted: file_count,
            total_scan_time: scan_time,
            total_delete_time: std::time::Duration::ZERO,
        });
    }

    let (broker, tx, rx) = Broker::new(tree);
    let broker = Arc::new(broker);

    let error_tracker = Arc::new(worker::ErrorTracker::new());
    let worker_config = worker::WorkerConfig {
        verbose: args.verbose,
        ignore_errors: args.ignore_errors,
    };

    if !args.silent {
        println!("Spawning {} worker threads...", worker_count);
    }
    let handles = worker::spawn_workers(
        worker_count,
        rx,
        broker.clone(),
        worker_config,
        error_tracker.clone(),
    );

    drop(tx);

    if !args.silent {
        println!("Deleting directories...");
    }
    let delete_start = Instant::now();

    let progress_handle = if !args.silent {
        let total = broker.total_dirs();
        let broker_clone = broker.clone();
        Some(std::thread::spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_millis(250));
            let completed = broker_clone.completed_count();
            if completed >= total {
                break;
            }
            let pct = (completed as f64 / total as f64 * 100.0) as u32;
            print!("\rDeleting... {}% ({}/{} dirs)", pct, completed, total);
            use std::io::Write;
            std::io::stdout().flush().ok();
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

    let failures = error_tracker.get_failures();
    let failure_count = failures.len();

    let stats = DeletionStats {
        dirs_deleted: dir_count,
        files_deleted: file_count,
        total_scan_time: scan_time,
        total_delete_time: delete_time,
    };

    if failure_count == 0 {
        println!("\nDeletion complete!");
        if args.stats {
            println!("\nStatistics:");
            println!("  Directories: {}", dir_count);
            println!("  Files:       {}", file_count);
            println!("  Total items: {}", dir_count + file_count);
            println!("\nTiming:");
            println!("  Scan time:   {:.2?}", scan_time);
            println!("  Delete time: {:.2?}", delete_time);
            println!("  Total time:  {:.2?}", total_time);
            println!("\nPerformance:");
            let items_per_sec = (dir_count + file_count) as f64 / total_time.as_secs_f64();
            println!("  Throughput:  {:.0} items/sec", items_per_sec);
        } else if !args.silent {
            println!("  Scan time:   {:.2?}", scan_time);
            println!("  Delete time: {:.2?}", delete_time);
            println!("  Total time:  {:.2?}", total_time);
        }
        Ok(stats)
    } else {
        println!("\nDeletion completed with errors!");
        if !args.silent {
            println!("  Scan time:   {:.2?}", scan_time);
            println!("  Delete time: {:.2?}", delete_time);
            println!("  Total time:  {:.2?}", total_time);
        }

        let total_completed = broker.completed_count();
        let total_items = total_completed + failure_count;

        println!("\nError Summary:");
        println!(
            "  {} of {} items failed to delete",
            failure_count, total_items
        );

        let display_count = std::cmp::min(10, failure_count);
        println!("\nFirst {} failures:", display_count);
        for (i, failure) in failures.iter().take(display_count).enumerate() {
            let item_type = if failure.is_dir { "dir" } else { "file" };
            println!(
                "  {}. [{}] {}: {}",
                i + 1,
                item_type,
                failure.path.display(),
                failure.error
            );
        }

        if failure_count > 10 {
            println!("\n  ... and {} more failures", failure_count - 10);
            println!("\nRun with --verbose to see all errors as they occur");
        }

        Err(Error::PartialFailure {
            total: total_items,
            failed: failure_count,
            errors: failures,
        })
    }
}
