#!/usr/bin/env node

const https = require('https');
const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

const REPO = 'mattew8/oh-my-worktree';
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

function downloadFile(url, dest) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest);

    const request = (url) => {
      https.get(url, (response) => {
        if (response.statusCode === 302 || response.statusCode === 301) {
          // Follow redirect
          request(response.headers.location);
          return;
        }

        if (response.statusCode !== 200) {
          reject(new Error(`Failed to download: ${response.statusCode}`));
          return;
        }

        response.pipe(file);
        file.on('finish', () => {
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

  console.log(`Downloading owt v${version} for ${process.platform}-${process.arch}...`);
  console.log(`URL: ${downloadUrl}`);

  try {
    await downloadFile(downloadUrl, binPath);

    // Make executable on Unix systems
    if (process.platform !== 'win32') {
      fs.chmodSync(binPath, 0o755);
    }

    console.log('owt installed successfully!');
  } catch (error) {
    console.error('Failed to download binary:', error.message);
    console.error('');
    console.error('You can manually install owt using cargo:');
    console.error('  cargo install --git https://github.com/mattew8/oh-my-worktree');
    process.exit(1);
  }
}

install();
