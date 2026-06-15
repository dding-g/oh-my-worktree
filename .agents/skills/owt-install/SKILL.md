---
name: owt-install
description: Use when setting up an AI agent environment for owt, verifying the owt CLI is available, installing oh-my-worktree, or preparing an agent to use owt worktree commands instead of raw git worktree commands.
---

# owt Install

Use this skill before any agent workflow that assumes `owt` is installed.

## Workflow

1. Verify:

```bash
command -v owt
owt --version
owt worktree --help
```

2. If missing, install with the least surprising option for the environment:

```bash
npm install -g oh-my-worktree
```

In an `oh-my-worktree` source checkout, prefer the local binary when you are validating unreleased changes:

```bash
cargo build --release
./target/release/owt --version
```

Use a source install only when global npm install is unavailable or not desired:

```bash
cargo install --git https://github.com/dding-g/oh-my-worktree.git
```

3. Verify the plain CLI:

```bash
owt worktree list
owt search main
```

4. For interactive human shell handoff, run `owt setup` and tell the user to reload their shell. For agent automation, use plain CLI commands instead of the TUI.

## Guardrails

- Do not drive `owt`'s TUI in scripted agent workflows.
- Do not silently fall back to mutating `git worktree` commands when `owt` is unavailable.
- If installation fails due permissions, network, or missing toolchains, report the blocker and ask for the narrow missing capability.
- After setup, use `.agents/skills/owt-worktree/SKILL.md` for worktree operations.
