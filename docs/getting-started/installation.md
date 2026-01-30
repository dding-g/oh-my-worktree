---
layout: default
title: Installation
parent: Getting Started
nav_order: 1
---

# Installation

There are several ways to install owt.

## npm (Recommended)

The easiest way to install owt is via npm:

```bash
npm install -g oh-my-worktree
```

You can also run it without installing using npx:

```bash
npx oh-my-worktree
```

## Cargo

If you have Rust installed, you can install via Cargo:

```bash
cargo install --git https://github.com/dding-g/oh-my-worktree
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

## Requirements

- **Git 2.5+** (worktree support)
- A bare repository (see [Quick Start](/oh-my-worktree/getting-started/quick-start))
