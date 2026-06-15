# Agent Bootstrap Prompt: Use owt for Worktrees

You are working in a repository that standardizes Git worktree handling through `owt` (`oh-my-worktree`). Before creating, deleting, listing, pruning, searching, or switching worktrees, install or verify `owt` and use its plain CLI.

Follow this setup:

1. Check `command -v owt` and `owt --version`.
2. If `owt` is missing, install it with the best available path:
   - Prefer `npm install -g oh-my-worktree` when Node/npm are available.
   - In an `oh-my-worktree` source checkout, use `cargo build --release` and run `target/release/owt` or put it on `PATH`.
   - Use `cargo install --git https://github.com/dding-g/oh-my-worktree.git` only when source install is acceptable.
3. Verify the agent-safe surface with `owt worktree --help` and `owt worktree list`.
4. Load or follow these skills when available:
   - `.agents/skills/owt-install/SKILL.md`
   - `.agents/skills/owt-worktree/SKILL.md`

Operational rules:

- Use `owt worktree list/create/delete/prune`, `owt search`, `owt pr status`, and `owt commit tree` for automation.
- Do not drive the TUI for scripted agent work.
- Do not use raw `git worktree add/remove/prune` for mutations unless `owt` cannot run and the user explicitly approves the fallback.
- Parse stdout as tab-separated records. `owt worktree create` prints `created<TAB>branch<TAB>path`.
- Use `--tmux=on` only when the user requests tmux pane creation or the task requires it.
- Use `--branch` on delete only when the user wants the local branch removed too. Use `--force` only after confirming dirty-worktree risk.
