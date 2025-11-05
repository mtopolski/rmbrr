# Changelog

## [0.1.0] - Unreleased

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
