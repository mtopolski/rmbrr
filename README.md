# rmbrr

Windows efficient rmdir with cross-platform compatibility.

## Performance

### Windows
Benchmark on node_modules (28,434 files, 5,122 directories, 350 MB):

| Method              | Time      | vs rmbrr |
|---------------------|-----------|----------|
| rmbrr               | 1,780ms   | 1.00x    |
| rimraf              | 3,175ms   | 1.78x slower |
| PowerShell          | 6,824ms   | 3.83x slower |
| cmd rmdir           | 6,422ms   | 3.61x slower |
| cmd del+rmdir       | 7,175ms   | 4.03x slower |
| robocopy /MIR       | 9,528ms   | 5.35x slower |

### Linux (Ubuntu, WSL)
Benchmark on node_modules (28,268 files, 5,124 directories, 446 MB):

| Method              | Time      | vs rmbrr |
|---------------------|-----------|----------|
| rmbrr               | 192ms     | 1.00x    |
| rimraf              | 660ms     | 3.44x slower |
| rm -rf              | 711ms     | 3.70x slower |

Test system: 16-core CPU, SSD. Default thread count (CPU cores).

### Macos (ARM)
Benchmark on node_modules (28,340 files, 5,126 directories, 413 MB):

| Method              | Time      | vs rmbrr |
|---------------------|-----------|----------|
| rmbrr               | 1,170ms   | 1.00x    |
| rimraf              | 1,270ms   | 1.09x slower |
| rm -rf              | 3,000ms   | 2.56x slower |

Test system: 2020 M1 Macbook Pro, SSD. Default thread count (CPU cores).

## Installation

### npm
```bash
npm install -g rmbrr
# or use directly
npx rmbrr ./node_modules
```

### Homebrew (macOS/Linux)
```bash
brew tap mtopolski/tap
brew install rmbrr
```

### Cargo
```bash
cargo install rmbrr
```

### Install script (Unix/Linux/macOS)
```bash
curl -fsSL https://raw.githubusercontent.com/mtopolski/rmbrr/main/install.sh | sh
```

### Install script (Windows)
```powershell
iwr -useb https://raw.githubusercontent.com/mtopolski/rmbrr/main/install.ps1 | iex
```

### Pre-built binaries
Download from [releases](https://github.com/mtopolski/rmbrr/releases).

## Usage

```bash
# Delete a directory
rmbrr path/to/directory

# Multiple directories
rmbrr dir1 dir2 dir3

# Dry run (scan only, don't delete)
rmbrr -n path/to/directory

# Ask for confirmation
rmbrr --confirm path/to/directory

# Show detailed statistics
rmbrr --stats path/to/directory

# Specify thread count
rmbrr --threads 8 path/to/directory

# Silent mode (disable progress for maximum performance)
rmbrr --silent path/to/directory
```

## How it works

### Windows (POSIX semantics)
- POSIX delete semantics via `SetFileInformationByHandle` with `FILE_DISPOSITION_FLAG_POSIX_SEMANTICS`
- Immediate namespace removal (files can be deleted while in use)
- Ignores readonly attributes automatically
- Direct Windows API calls (FindFirstFileExW for enumeration)
- Parallel deletion with dependency-aware scheduling
- Bottom-up traversal (delete files/subdirs before parent dirs)
- Long path support (\\?\ prefix)

### Unix/Linux
- Standard library `remove_file`/`remove_dir` calls
- Same parallel deletion architecture

## Requirements

- Windows: Windows 10 1607+ with NTFS filesystem
- Unix/Linux: Any modern system

## License

MIT OR Apache-2.0
