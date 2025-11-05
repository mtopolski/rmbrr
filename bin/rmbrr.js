#!/usr/bin/env node

const { spawnSync } = require('child_process');
const { join } = require('path');

// Determine binary name
const platform = process.platform;
const binaryName = platform === 'win32' ? 'rmbrr.exe' : 'rmbrr';
const binaryPath = join(__dirname, binaryName);

// Execute the binary with all arguments
const result = spawnSync(binaryPath, process.argv.slice(2), {
  stdio: 'inherit',
  shell: false
});

process.exit(result.status || 0);
