#!/usr/bin/env node

const { spawnSync } = require('child_process');
const { existsSync, chmodSync, mkdirSync } = require('fs');
const { join } = require('path');
const https = require('https');
const { createWriteStream } = require('fs');

const REPO = 'mtopolski/rmbrr';
const VERSION = require('../package.json').version;

// Determine platform and architecture
const platform = process.platform;
const arch = process.arch;

let target;
let binaryName = 'rmbrr';

if (platform === 'win32' && arch === 'x64') {
  target = 'windows-x86_64';
  binaryName = 'rmbrr.exe';
} else if (platform === 'darwin' && arch === 'x64') {
  target = 'macos-x86_64';
} else if (platform === 'darwin' && arch === 'arm64') {
  target = 'macos-aarch64';
} else if (platform === 'linux' && arch === 'x64') {
  target = 'linux-x86_64';
} else {
  console.error(`Unsupported platform: ${platform}-${arch}`);
  console.error('Please install rmbrr manually from: https://github.com/mtopolski/rmbrr/releases');
  process.exit(1);
}

const binaryDir = join(__dirname, '..', 'bin');
const binaryPath = join(binaryDir, binaryName);

// Skip if already installed
if (existsSync(binaryPath)) {
  console.log('rmbrr binary already exists, skipping download');
  process.exit(0);
}

console.log(`Downloading rmbrr v${VERSION} for ${platform}-${arch}...`);

// Ensure bin directory exists
if (!existsSync(binaryDir)) {
  mkdirSync(binaryDir, { recursive: true });
}

// Construct download URL
const downloadUrl = `https://github.com/${REPO}/releases/download/v${VERSION}/rmbrr-${target}${platform === 'win32' ? '.exe' : ''}`;

console.log(`Downloading from: ${downloadUrl}`);

// Download binary
const file = createWriteStream(binaryPath);
https.get(downloadUrl, (response) => {
  if (response.statusCode === 302 || response.statusCode === 301) {
    // Follow redirect
    https.get(response.headers.location, (res) => {
      res.pipe(file);
      file.on('finish', () => {
        file.close(() => {
          // Make executable on Unix
          if (platform !== 'win32') {
            chmodSync(binaryPath, 0o755);
          }
          console.log('Successfully installed rmbrr!');
          console.log('');
          console.log('Try it out:');
          console.log('  npx rmbrr --help');
        });
      });
    });
  } else if (response.statusCode === 200) {
    response.pipe(file);
    file.on('finish', () => {
      file.close(() => {
        // Make executable on Unix
        if (platform !== 'win32') {
          chmodSync(binaryPath, 0o755);
        }
        console.log('Successfully installed rmbrr!');
        console.log('');
        console.log('Try it out:');
        console.log('  npx rmbrr --help');
      });
    });
  } else {
    console.error(`Failed to download: HTTP ${response.statusCode}`);
    console.error('Please install manually from: https://github.com/mtopolski/rmbrr/releases');
    process.exit(1);
  }
}).on('error', (err) => {
  console.error(`Download failed: ${err.message}`);
  console.error('Please install manually from: https://github.com/mtopolski/rmbrr/releases');
  process.exit(1);
});
