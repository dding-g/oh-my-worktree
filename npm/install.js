#!/usr/bin/env node

const https = require('https');
const fs = require('fs');
const path = require('path');
const { execSync, spawn } = require('child_process');

const REPO = 'dding-g/oh-my-worktree';
const BINARY_NAME = 'owt';

function getPlatform() {
  const platform = process.platform;
  const arch = process.arch;

  const platformMap = {
    'darwin-x64': 'owt-darwin-x64',
    'darwin-arm64': 'owt-darwin-arm64',
    'linux-x64': 'owt-linux-x64',
    'linux-arm64': 'owt-linux-arm64',
    'win32-x64': 'owt-win32-x64.exe',
  };

  const key = `${platform}-${arch}`;
  const binaryName = platformMap[key];

  if (!binaryName) {
    console.error(`Unsupported platform: ${platform}-${arch}`);
    console.error('Supported platforms: darwin-x64, darwin-arm64, linux-x64, linux-arm64, win32-x64');
    process.exit(1);
  }

  return binaryName;
}

function getPackageVersion() {
  const packageJson = require('./package.json');
  return packageJson.version;
}

function commandExists(cmd) {
  try {
    execSync(`which ${cmd}`, { stdio: 'ignore' });
    return true;
  } catch {
    return false;
  }
}

// Fast download using curl (if available)
function downloadWithCurl(url, dest) {
  return new Promise((resolve, reject) => {
    const curl = spawn('curl', ['-fSL', '--progress-bar', '-o', dest, url], {
      stdio: ['ignore', 'inherit', 'inherit']
    });
    curl.on('close', (code) => {
      if (code === 0) resolve();
      else reject(new Error(`curl exited with code ${code}`));
    });
    curl.on('error', reject);
  });
}

// Fallback: Node.js https with progress
function downloadWithNode(url, dest) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest);

    const request = (url) => {
      https.get(url, (response) => {
        if (response.statusCode === 302 || response.statusCode === 301) {
          request(response.headers.location);
          return;
        }

        if (response.statusCode !== 200) {
          reject(new Error(`HTTP ${response.statusCode}`));
          return;
        }

        const totalSize = parseInt(response.headers['content-length'], 10);
        let downloaded = 0;

        response.on('data', (chunk) => {
          downloaded += chunk.length;
          if (totalSize) {
            const percent = Math.round((downloaded / totalSize) * 100);
            process.stdout.write(`\rDownloading... ${percent}%`);
          }
        });

        response.pipe(file);
        file.on('finish', () => {
          process.stdout.write('\n');
          file.close();
          resolve();
        });
      }).on('error', (err) => {
        fs.unlink(dest, () => {});
        reject(err);
      });
    };

    request(url);
  });
}

async function install() {
  const binaryName = getPlatform();
  const version = getPackageVersion();
  const binDir = path.join(__dirname, 'bin');
  const binPath = path.join(binDir, BINARY_NAME + (process.platform === 'win32' ? '.exe' : ''));

  // Create bin directory
  if (!fs.existsSync(binDir)) {
    fs.mkdirSync(binDir, { recursive: true });
  }

  const downloadUrl = `https://github.com/${REPO}/releases/download/v${version}/${binaryName}`;

  console.log(`Installing owt v${version} for ${process.platform}-${process.arch}`);

  try {
    // Prefer curl for faster download with built-in progress
    if (commandExists('curl')) {
      await downloadWithCurl(downloadUrl, binPath);
    } else {
      await downloadWithNode(downloadUrl, binPath);
    }

    // Make executable on Unix systems
    if (process.platform !== 'win32') {
      fs.chmodSync(binPath, 0o755);
    }

    console.log('');
    console.log('✓ owt installed successfully!');
    console.log('');
    console.log(`  Version:   v${version}`);
    console.log(`  Changelog: https://github.com/${REPO}/releases/tag/v${version}`);
    console.log('');
  } catch (error) {
    console.error('Failed to download binary:', error.message);
    console.error('');
    console.error('Alternative installation methods:');
    console.error('  cargo install --git https://github.com/dding-g/oh-my-worktree');
    process.exit(1);
  }
}

install();
