# win-rmdir-fast

Fast parallel directory deletion for Windows.

## Performance

Benchmark on a modest node_modules (43,329 files, 4,349 directories):

| Method              | Time      | vs win-rmdir-fast |
|---------------------|-----------|-------------------|
| win-rmdir-fast      | 4,238ms   | 1.00x             |
| rimraf              | 5,867ms   | 1.38x slower      |
| PowerShell          | 8,536ms   | 2.01x slower      |
| cmd del+rmdir       | 10,041ms  | 2.37x slower      |
| cmd rd              | 10,651ms  | 2.51x slower      |
| robocopy /MIR       | 15,367ms  | 3.63x slower      |

## Installation

```bash
cargo install win-rmdir-fast
```

Or download pre-built binaries from [releases](https://github.com/mtopolski/win-rmdir-fast/releases).

## Usage

```bash
# Delete a directory
win-rmdir-fast path/to/directory

# Dry run (scan only, don't delete)
win-rmdir-fast --dry-run path/to/directory

# Specify thread count
win-rmdir-fast --threads 8 path/to/directory
```

## How it works

- Direct Windows API calls (DeleteFileW, RemoveDirectoryW, FindFirstFileExW)
- Parallel deletion with dependency-aware scheduling
- Bottom-up traversal (delete files/subdirs before parent dirs)
- Lazy attribute clearing (only on access denied)
- Long path support (\\?\ prefix)

## Requirements

Windows only.

## License

MIT OR Apache-2.0
