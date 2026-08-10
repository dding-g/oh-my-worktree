#!/usr/bin/env node

const fs = require('fs');
const path = require('path');

const root = path.resolve(__dirname, '..');
const checkOnly = process.argv.includes('--check');
const requestedVersionArg = process.argv.find((arg) => /^v?\d+\.\d+\.\d+$/.test(arg));

function read(relativePath) {
  return fs.readFileSync(path.join(root, relativePath), 'utf8');
}

function cargoVersion() {
  const match = read('Cargo.toml').match(/^version = "([^"]+)"/m);
  if (!match) throw new Error('Could not read package version from Cargo.toml');
  return match[1];
}

const version = requestedVersionArg
  ? requestedVersionArg.replace(/^v/, '')
  : cargoVersion();
if (!/^\d+\.\d+\.\d+$/.test(version)) {
  throw new Error(`Invalid release version: ${version}`);
}

const updates = new Map();

function stage(relativePath, transform) {
  const current = read(relativePath);
  const next = transform(current);
  if (current !== next) updates.set(relativePath, next);
}

function replaceRequired(current, pattern, replacement, description) {
  if (!pattern.test(current)) {
    throw new Error(`Could not find ${description}`);
  }
  return current.replace(pattern, replacement);
}

function updateJsonVersion(relativePath) {
  stage(relativePath, (current) => {
    const value = JSON.parse(current);
    value.version = version;
    return `${JSON.stringify(value, null, 2)}\n`;
  });
}

stage('Cargo.toml', (current) =>
  replaceRequired(
    current,
    /^version = "[^"]+"/m,
    `version = "${version}"`,
    'package version in Cargo.toml'
  )
);
stage('Cargo.lock', (current) =>
  replaceRequired(
    current,
    /(\[\[package\]\]\nname = "oh-my-worktree"\nversion = ")[^"]+"/,
    `$1${version}"`,
    'oh-my-worktree package version in Cargo.lock'
  )
);
updateJsonVersion('package.json');
updateJsonVersion('npm/package.json');
stage('docs/_config.yml', (current) =>
  replaceRequired(
    current,
    /^owt_version: "[^"]+"/m,
    `owt_version: "${version}"`,
    'owt_version in docs/_config.yml'
  )
);

const pinnedDocs = [
  'README.md',
  'README.ko.md',
  'npm/README.md',
  'docs/index.md',
  'docs/index.html',
  'docs/getting-started/installation.md',
  '.agents/skills/owt-install/SKILL.md',
];

for (const relativePath of pinnedDocs) {
  stage(relativePath, (current) => {
    const pattern = /v\d+\.\d+\.\d+/g;
    return replaceRequired(
      current,
      pattern,
      `v${version}`,
      `pinned version in ${relativePath}`
    );
  });
}

if (updates.size === 0) {
  console.log(`release version ${version} is synchronized`);
  process.exit(0);
}

if (checkOnly) {
  console.error(`release version ${version} is stale in:`);
  for (const relativePath of updates.keys()) console.error(`- ${relativePath}`);
  process.exit(1);
}

for (const [relativePath, content] of updates) {
  fs.writeFileSync(path.join(root, relativePath), content);
  console.log(`updated ${relativePath} -> ${version}`);
}
