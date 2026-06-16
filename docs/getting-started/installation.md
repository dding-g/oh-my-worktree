---
layout: default
title: Installation
parent: Getting Started
nav_order: 1
---

# Installation

There are several ways to install owt.

## Latest Release (Recommended)

If you have Rust installed, install the current release directly from the Git tag:

```bash
cargo install --git https://github.com/dding-g/oh-my-worktree --tag v0.13.0 --force
```

## Prebuilt Binaries

Prebuilt binaries are attached to the latest GitHub Release:

<https://github.com/dding-g/oh-my-worktree/releases/latest>

For macOS Apple Silicon:

```bash
mkdir -p ~/.local/bin
curl -L https://github.com/dding-g/oh-my-worktree/releases/latest/download/owt-darwin-arm64 -o ~/.local/bin/owt
chmod +x ~/.local/bin/owt
```

Use `owt-darwin-x64`, `owt-linux-x64`, `owt-linux-arm64`, or `owt-win32-x64.exe` for other platforms.

## npm Wrapper

When the npm registry package is current, the npm wrapper downloads the matching release binary:

```bash
npm install -g oh-my-worktree
```

## Build from Source

Clone the repository and build:

```bash
git clone https://github.com/dding-g/oh-my-worktree.git
cd oh-my-worktree
cargo build --release
```

The binary will be at `./target/release/owt`.

## Verify Installation

After installation, verify owt is working:

```bash
owt --version
```

The current release prints `owt v0.13.0`.

## Requirements

- **Git 2.5+** (worktree support)
- A regular Git repository or a bare repository layout. The `.bare` layout is recommended when you want project-local sibling worktrees.
