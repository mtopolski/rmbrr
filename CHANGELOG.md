# Changelog

## [0.1.6] - 2025-01-05

### Fixed
- Growing pains with publishing to 3 targets

## [0.1.1] - 2025-01-05

### Fixed
- Removed unnecessary type casts in Windows API error handling
- Cleaned up low-value code comments
- Fixed clippy warnings for all platforms

### Changed
- Removed time estimation from dry-run output (unreliable prediction)

## [0.1.0] - 2025-01-05

Initial release.

### Features
- Fast parallel directory deletion
- Windows POSIX semantics for immediate namespace removal
- Cross-platform support (Windows, Linux, macOS)
- Progress reporting with `--silent` flag
- Dry-run mode with `-n` / `--dry-run`
- Multi-path deletion support
- Confirmation prompt with `--confirm`
- Detailed statistics with `--stats`
- Safety checks for system directories
- Custom error handling with `--verbose` flag
- Thread count configuration with `--threads`

### Performance
- 1.8-5.5x faster than competing tools on Windows
- Excellent thread scaling up to 16+ cores
- Automatic CPU detection for optimal thread count
